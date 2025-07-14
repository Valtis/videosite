mod db;
mod model;

use std::env;

use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_sqs::Client;

use tracing_subscriber::filter;
use serde_json;
use tower::ServiceBuilder;


use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    middleware::from_fn,
    response::IntoResponse,
    routing::{get},
    Router,
};


use db::*;

use auth_check::{auth_middleware, UserInfo};

struct ResourceStatusUpdateEvent {
    pub message: ResourceStatusUpdateMessage,
    pub receipt_handle: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "status")]
enum ResourceStatusUpdateMessage {
    #[serde(rename = "uploaded")]
    ResourceUploaded {
        user_id: String,
        object_name: String,
        file_name: String,
        origin_file_path: String,
    },
    #[serde(rename = "failed")]
    ResourceProcessingFailed {
        object_name: String,
    },
    #[serde(rename = "processing")]
    ResourceProcessingStarted {
        object_name: String,
    },
    #[serde(rename = "type_resolved")]
    ResourceTypeResolved {
        object_name: String,
        resource_type: String,
    },
    #[serde(rename = "processed")]
    ResourceProcessed {
        object_name: String,
        storage_path: String,
    },
}


/// List resources of the current user.
/// 
/// Returns a JSON array of resources.
/// If no resources are found, returns an empty array.
/// 
/// # Arguments
/// * `user_info` - The user information extracted from the request, containing the user ID
/// 
/// Returns a tuple containing the HTTP status code and a JSON response with the list of resources.
/// 
async fn list_resources(user_info: Extension<UserInfo>) -> impl IntoResponse{
    let resources = db::get_active_resources_by_user_id(&user_info.user_id);
    if resources.is_empty() {
        return (StatusCode::OK, Json(vec![]));
    }
    let resources: Vec<model::Resource> = resources.into_iter().map(model::Resource::from).collect();
    (StatusCode::OK, Json(resources))
}


async fn resource_status_listener() {
    let queue_url = env::var("RESOURCE_STATUS_QUEUE_URL").expect("RESOURCE_STATUS_QUEUE_URL not set");
    let client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);

    loop {
        let resource_status_update_opt = receive_resource_status_update_message(&client, &queue_url).await
            .unwrap_or_else(|err| {
                tracing::error!("Error receiving resource status update: {}", err);
                None
            });
        
        if let Some(resource_status_update) = resource_status_update_opt {
            tracing::info!("Received resource status update: {:?}", resource_status_update.message);


            match resource_status_update.message {
                ResourceStatusUpdateMessage::ResourceUploaded { user_id, object_name, file_name, origin_file_path } => {
                    create_resource(
                        object_name,
                        user_id, 
                        file_name,
                        origin_file_path,
                    );
                },
                ResourceStatusUpdateMessage::ResourceProcessingFailed { object_name } => {
                    update_resource_status(object_name, "failed".to_string());
                },
                ResourceStatusUpdateMessage::ResourceProcessingStarted { object_name } => {
                    update_resource_status(object_name, "processing".to_string());
                },
                ResourceStatusUpdateMessage::ResourceTypeResolved { object_name, resource_type } => {
                    update_resource_type(object_name, resource_type);
                },
                ResourceStatusUpdateMessage::ResourceProcessed { object_name, storage_path } => {
                    update_resource_status(object_name.clone(), "processed".to_string());
                    update_resource_storage(object_name, storage_path);
                },
            };        



            delete_message(&client, &queue_url, &resource_status_update.receipt_handle)
                .await
                .unwrap_or_else(|err| {
                    tracing::error!("Error deleting message: {}", err);
                });
        }
        // await 5 seconds before checking for new events
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

}

async fn receive_resource_status_update_message(client: &Client, queue_url: &str) -> Result<Option<ResourceStatusUpdateEvent>, aws_sdk_sqs::Error> {
    let rcv_message_output = client
        .receive_message()
        .queue_url(queue_url)
        .max_number_of_messages(1)
        .send()
        .await?;

    
    for message in rcv_message_output.messages.unwrap_or_default() {

        let body = match message.body {
            Some(body) => body,
            None => {
                tracing::warn!("Received message with no body, skipping.");
                continue;
            }
        };

        let resource_update_message: ResourceStatusUpdateMessage = match serde_json::from_str(&body){
            Ok(msg) => msg,
            Err(err) => {
                tracing::error!("Failed to parse message body as JSON: {}", err);
                continue;
            }
        };

        return Ok(Some(ResourceStatusUpdateEvent {
            message: resource_update_message,
            receipt_handle: message.receipt_handle.unwrap_or_default(),
        }));  

    }

    Ok(None)
}


async fn delete_message(client: &Client, queue_url: &str, receipt_handle: &str) -> Result<(), aws_sdk_sqs::Error> {
    client
        .delete_message()
        .queue_url(queue_url)
        .receipt_handle(receipt_handle)
        .send()
        .await?;

    tracing::info!("Message deleted successfully");
    Ok(())
}

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_level(true)
        .pretty()
        .with_max_level(filter::LevelFilter::INFO)
        .init();

    let app = Router::new()
        .route("/resource/health", get(|| async { "OK" }))
        .route("/resource/list", get(list_resources))
            .layer(
                ServiceBuilder::new()
                    .layer(from_fn(auth_middleware))
            );
        
        

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await
        .expect("Failed to bind TCP listener");

    let resource_status_listener_task = resource_status_listener();

    tokio::join!(
        resource_status_listener_task,
        axum::serve(listener, app)
    ).1.unwrap();
}