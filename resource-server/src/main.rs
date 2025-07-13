mod db;

use std::env;

use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_sqs::Client;

use tracing_subscriber::filter;
use serde_json;


use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};


use db::create_resource;


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
    Placeholder
}

async fn list_resources() -> impl IntoResponse{
    (StatusCode::OK, "TODO!")
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
                ResourceStatusUpdateMessage::Placeholder => {
                    tracing::warn!("Received placeholder message, no action taken.");
                }
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
        ;
        

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await
        .expect("Failed to bind TCP listener");

    let resource_status_listener_task = resource_status_listener();

    tokio::join!(
        resource_status_listener_task,
        axum::serve(listener, app)
    ).1.unwrap();
}