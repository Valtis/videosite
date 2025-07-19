mod db;
mod model;

use std::env;

use aws_sdk_s3::{Client as S3Client};
use aws_sdk_s3::error::DisplayErrorContext;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_sqs::Client;

use tracing_subscriber::filter;
use serde_json;
use tokio_util::io::ReaderStream;
use tower::ServiceBuilder;


use axum::{
    body::{Body}, 
    extract::{Extension, Json}, 
    http::{StatusCode},
    middleware::from_fn, 
    response::{IntoResponse},
    routing::{get, post}, 
    Router
};

use axum_client_ip::{ ClientIp, ClientIpSource };

use http_body_util::StreamBody;

use db::*;
use model::*;

use auth_check::{auth_middleware, add_user_info_to_request, UserInfo};
use audit::{AuditEvent, send_audit_event};

const RESOURCE_BUCKET: &'static str = "resource";

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

/// Get the master playlist for a video resource.
/// 
/// 
#[axum::debug_handler]
async fn get_video_master_playlist(
    user_info: Extension<Option<UserInfo>>,
    params: axum::extract::Path<String>,
) -> impl IntoResponse {
    let resource_id = params.0;
    send_resource(user_info.0, resource_id, "master.m3u8".to_string(), "video").await
}

#[axum::debug_handler]
async fn get_stream_asset(
    user_info: Extension<Option<UserInfo>>,
    params: axum::extract::Path<(String, String, String)>,
) -> impl IntoResponse {
    let resource_id = params.0.0;
    let index = params.0.1;
    let file_name = params.0.2;

    let file_in_directory = format!("stream_{}/{}", index, file_name);
    send_resource(user_info.0, resource_id, file_in_directory, &"video", ).await
}

#[axum::debug_handler]
async fn update_resource_public_status(
    user_info: Extension<UserInfo>,
    params: axum::extract::Path<String>,
    ClientIp(client_ip): ClientIp,
    Json(update): Json<ResourcePublicStatusUpdate>,
) -> impl IntoResponse {
    let resource_id = params.0;
    let resource = db::get_active_resource_by_id(&resource_id);

    if let Some(resource) = resource {
        if resource.user_id.to_string() == user_info.user_id {
            db::update_resource_public_status(&resource_id, update.is_public);

            send_audit_event(AuditEvent {
                event_type: "resource_public_status_updated".to_string(),
                user_id: Some(&user_info.user_id),
                client_ip: &client_ip.to_string(),
                target: Some(&resource_id),
                event_details: Some(serde_json::json!({
                    "is_public": update.is_public,
                })),
            }).await.unwrap_or_else(|err| {
                tracing::error!("Failed to send audit event: {}", err);
            });

            return StatusCode::OK
        }
    }

    return StatusCode::NOT_FOUND;
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
                ResourceStatusUpdateMessage::ResourceUploaded { user_id, object_name, file_name} => {
                    create_resource(
                        object_name,
                        user_id, 
                        file_name,
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
                ResourceStatusUpdateMessage::ResourceProcessed { object_name} => {
                    update_resource_status(object_name.clone(), "processed".to_string());
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

    let ip_source_env = env::var("IP_SOURCE").unwrap_or_else(|_| "nginx".to_string());
    let ip_source = match ip_source_env.as_str() {
        "nginx" => ClientIpSource::RightmostXForwardedFor,
        "amazon" => ClientIpSource::CloudFrontViewerAddress,
        _ => { 
            tracing::warn!("Unknown IP source: {}, defaulting to Nginx", ip_source_env);
            ClientIpSource::RightmostXForwardedFor
        } 
    };

    tracing_subscriber::fmt()
        .with_level(true)
        .pretty()
        .with_max_level(filter::LevelFilter::INFO)
        .init();

    let app = Router::new()
        .route("/resource/health", get(|| async { "OK" }))
        .nest(
            "/resource",
            Router::new()
                .route("/list", get(list_resources))
                .route("/{resource_id}/public", post(update_resource_public_status))
            .layer(
                ServiceBuilder::new()
                    .layer(from_fn(auth_middleware))
            )
        )
        .nest(
            "/resource/{resource_id}",
            Router::new()
                .route("/master.m3u8", get(get_video_master_playlist))
                .route("/stream_{index}/{file_in_directory}", get(get_stream_asset))
                .layer(
                ServiceBuilder::new()
                    .layer(from_fn(add_user_info_to_request))
            )
        ).layer(ip_source.into_extension());
        
        

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await
        .expect("Failed to bind TCP listener");

    let resource_status_listener_task = resource_status_listener();

    tokio::join!(
        resource_status_listener_task,
        axum::serve(
            listener, 
            app)
    ).1.unwrap();
}


async fn send_resource(
    user_info: Option<UserInfo>,
    resource_id: String, 
    file_in_directory: String,
    resource_type: &str,
) -> impl IntoResponse {
    tracing::info!("Sending resource {} for user {:?}", resource_id, user_info);
    let resource = db::get_active_resource_by_id(&resource_id);
    if let Some(resource) = resource {
        if resource.resource_type == resource_type && has_access_to_resource(&user_info, &resource) {

            if transfer_quota_exceeded() {
                tracing::warn!("Transfer quota exceeded for user {}", user_info.as_ref().map_or("unknown", |u| &u.user_id));
                // Hey, bandwidth is expensive. 
                return StatusCode::PAYMENT_REQUIRED.into_response();
            }

            let object_name = format!("{}/{}", resource.id, file_in_directory);
            let s3_client = get_s3_client().await;
          

            match get_object_stream(
                s3_client,
                RESOURCE_BUCKET,
                &object_name,
            ).await {
                Ok((stream, file_size)) => {
                    update_quota_used(file_size as i64).unwrap_or_else(|err| {
                        tracing::error!("Failed to update transfer quota: {}", err);
                    });
                    
                    let reader_stream = ReaderStream::new(stream.into_async_read());
                    return (StatusCode::OK, Body::from_stream(StreamBody::new(reader_stream))).into_response();
                },
                Err(err) => {
                    tracing::error!("Failed to get object stream: {}", err);
                    // most likely cause is that the object does not exist
                    return StatusCode::NOT_FOUND.into_response();
                }
            }

        } else {
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

fn transfer_quota_exceeded() -> bool {
    if let Ok(var) = env::var("ENABLE_DATA_QUOTAS") {
        if var.to_lowercase() == "true" {
            let daily_quota_mb: i64 = env::var("DAILY_DATA_QUOTA_MEGABYTES")
                .unwrap_or_else(|_| "1024".to_string())
                .parse()
                .unwrap_or(1024);

            let daily_quota_bytes = daily_quota_mb * 1024 * 1024;

            let used_quota = db::get_used_daily_quota().unwrap_or(0);
            return used_quota > daily_quota_bytes;
        }
    }
    false
}

fn update_quota_used(amount: i64) -> Result<(), String> {
    if transfer_quota_exceeded() {
        return Err("Transfer quota exceeded".to_string());
    }

    db::update_daily_quota(amount)
        .map_err(|err| format!("Failed to update transfer quota: {}", err))
}

fn has_access_to_resource(user_info: &Option<UserInfo>, resource: &db::Resource) -> bool {
    if resource.is_public {
        return true;
    }

    if let Some(user_info) = user_info {
        return user_info.user_id == resource.user_id.to_string();
    }

    false
}

async fn get_s3_client() -> S3Client {
    let config = aws_config::load_from_env().await;
    let client = S3Client::new(&config);

    let client = if let Ok(var) = env::var("USE_PATH_STYLE_BUCKETS") {
        if var.to_lowercase() == "true" {
            let config_builder = client.config().clone().to_builder();
            S3Client::from_conf(config_builder.force_path_style(true).build())
        } else {
            client
        }
    } else {
        client
    };

    client
}


async fn get_object_stream(
    s3_client: S3Client,
    bucket: &str,
    object_name: &str,
) -> Result<(ByteStream, i64), DisplayErrorContext<impl std::error::Error>> {
    tracing::info!("Getting object stream for bucket: {}, object: {}", bucket, object_name);
    let get_object_output = match s3_client
        .get_object()
        .bucket(bucket)
        .key(object_name)
        .send()
        .await {
            Ok(output) => output,
            Err(err) => {
                let error_with_context = DisplayErrorContext(err);
                return Err(error_with_context);
            }
        };

    if get_object_output.content_length.is_none() {
        tracing::warn!("Object {} in bucket {} has no content length", object_name, bucket);
    }

    Ok((get_object_output.body, get_object_output.content_length.unwrap_or(0)))
}