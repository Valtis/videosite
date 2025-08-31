mod db;
mod model;

use std::env;

use aws_sdk_s3::{Client as S3Client};
use aws_sdk_s3::error::DisplayErrorContext;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_sqs::Client;
use aws_smithy_types_convert::date_time::DateTimeExt;

use tracing_subscriber::filter;
use serde_json;
use tokio_util::io::ReaderStream;
use tower::ServiceBuilder;


use axum::{
    body::{Body}, 
    extract::{Extension, Json, Query}, 
    http::{StatusCode},
    middleware::from_fn, 
    response::{IntoResponse},
    routing::{get, post}, 
    Router
};

use axum_client_ip::{ ClientIp, ClientIpSource };

use http_body_util::StreamBody;


use url::Url;

use db::*;
use model::*;

use auth_check::{auth_middleware, add_user_info_to_request, UserInfo};
use audit::{AuditEvent, send_audit_event};

const RESOURCE_FOLDER: &str = "resource";

fn s3_bucket() -> String {
    env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set")
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
async fn get_video_thumnail(
    user_info: Extension<Option<UserInfo>>,
    params: axum::extract::Path<String>,
) -> impl IntoResponse {
    let resource_id = params.0;
    send_resource(user_info.0, resource_id, "thumbnail.jpg".to_string(), "video").await
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


#[axum::debug_handler]
async fn oembed_response(
    query_params: Query<std::collections::HashMap<String, String>>,
    user_info: Extension<Option<UserInfo>>,
) -> impl IntoResponse {

    let url_param = query_params.get("url");
    if url_param.is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // we expect the url to be in the format https://domain/resource/player.html?resource_id={resource_id}
    let url = url_param.unwrap();
    let parsed_url = Url::parse(url);
    if parsed_url.is_err() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let parsed_url = parsed_url.unwrap();
    let query_pairs = parsed_url.query_pairs();
    let resource_id_opt = query_pairs.into_owned().find(|(key, _)| key == "resource_id").map(|(_, value)| value);
    if resource_id_opt.is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let resource_id = resource_id_opt.unwrap();
    

    let resource = db::get_active_resource_by_id(&resource_id);
    if let Some(resource) = resource {
        // TODO: Images and audio have not been implemented yet
        if resource.resource_type != "video" || !has_access_to_resource(&user_info.0, &resource) {
            return StatusCode::NOT_FOUND.into_response();
        }

        let domain = env::var("DOMAIN_URL").expect("DOMAIN_URL must be set");

        let video_metadata = db::get_highest_quality_video_metadata(&resource_id);
        if video_metadata.is_none() {
            return StatusCode::NOT_FOUND.into_response();
        }
        let video_metadata = video_metadata.unwrap();

        let iframe_link = format!(
            r#"<iframe width="{width}" height="{height}" src="{domain}player.html?resource_id={resource_id}" frameborder="0" allow="autoplay; picture-in-picture" allowfullscreen></iframe>"#,
            width=video_metadata.width,
            height=video_metadata.height,
            domain = domain,
            resource_id = resource.id
        );

        let oembed = model::OEmbedResponse {
            version: "1.0".to_string(),
            title: resource.resource_name.clone(),
            author_name: None, // TODO - implement fetching user info internally from auth service
            author_url: None, // likewise
            provider_name: Some("Hipsutuubi".to_string()),
            provider_url: Some(domain.clone()),
            cache_age: Some(3600), // 1 hour
            thumbnail_url: Some(format!("{}/resource/{}/thumbnail.jpg", domain, resource.id)),
            thumbnail_width: None,
            thumbnail_height: None,
            resource: model::OEmbedResourceType::Video {
                html: iframe_link,
                width: video_metadata.width,
                height: video_metadata.height,
            }
        };

        return (StatusCode::OK, Json(oembed)).into_response();

    }
    tracing::info!("DEBUG: Resource not found");
    StatusCode::NOT_FOUND.into_response()
}

#[axum::debug_handler]
async fn resource_metadata(
    user_info: Extension<Option<UserInfo>>,
    params: axum::extract::Path<String>,
) -> impl IntoResponse {
    let resource_id = params.0;
    let resource = db::get_active_resource_by_id(&resource_id);

    if let Some(resource) = resource {
        if !has_access_to_resource(&user_info.0, &resource) {
            tracing::info!("DEBUG: No access to resource");
            return StatusCode::NOT_FOUND.into_response();
        }
        let resource_metadata = match resource.resource_type.as_str() {
            "video" => {
                let video_metadata = db::get_highest_quality_video_metadata(&resource_id);
                if let Some(video_metadata) = video_metadata {
                    model::ResourceMetadata::Video {
                        width: video_metadata.width,
                        height: video_metadata.height,
                        duration_seconds: video_metadata.duration_seconds,
                        bit_rate: video_metadata.bit_rate,
                        frame_rate: video_metadata.frame_rate,
                    }
                } else {
                    tracing::info!("DEBUG: No video metadata found for resource {}", resource_id);
                    return StatusCode::NOT_FOUND.into_response();
                }
            },
            "audio" => {
                tracing::warn!("Audio resources are not yet supported");
                return StatusCode::NOT_FOUND.into_response();
            },
            "image" => {
                tracing::warn!("Image resources are not yet supported");
                return StatusCode::NOT_FOUND.into_response();
            },
            _ => {
                tracing::error!("Unknown resource type: {}", resource.resource_type);
                return StatusCode::NOT_FOUND.into_response();
            },
        };

        let response = model::ResourceMetadataResponse {
            id: resource.id.to_string(),
            name: resource.resource_name,
            status: resource.resource_status,
            resource_metadata,
        };

        return (StatusCode::OK, Json(response)).into_response(); 
    }

    tracing::info!("DEBUG: Resource {} not found", resource_id);
    StatusCode::NOT_FOUND.into_response()
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
                            update_resource_type(object_name.clone(),  resource_type.to_string()); 
                } 
                ResourceStatusUpdateMessage::ResourceProcessed { object_name, metadata} => {
                    update_resource_status(object_name.clone(), "processed".to_string());
                    match metadata {
                        model::ProducedResourceMetadata::Video(
                            quality_versions                            
                        ) => {

                            for video_data in &quality_versions {
                                db::insert_video_metadata(
                                    &object_name, 
                                    video_data.width, 
                                    video_data.height, 
                                    video_data.duration, 
                                    video_data.bitrate, 
                                    video_data.frame_rate);
                            }
                        },
                        model::ProducedResourceMetadata::Audio(_) => {
                            tracing::warn!("Audio files are not yet supported");
                        },
                        model::ProducedResourceMetadata::Image(_)=> {
                            tracing::warn!("Image files are not yet supported");
                        },
                    }
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
                tracing::error!("Failed to parse message body as JSON: {} (message: {})", err, body);
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

    tracing::info!("Starting resource server...");

    let ip_source_env = env::var("IP_SOURCE").unwrap_or_else(|_| "nginx".to_string());
    let ip_source = match ip_source_env.as_str() {
        "nginx" => ClientIpSource::RightmostXForwardedFor,
        "amazon" => ClientIpSource::CloudFrontViewerAddress,
        "cloudflare" => ClientIpSource::CfConnectingIp,
        _ => { 
            tracing::warn!("Unknown IP source: {}, defaulting to Nginx", ip_source_env);
            ClientIpSource::RightmostXForwardedFor
        } 
    };

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
            "/resource",
            Router::new()
                .route("/oembed.json", get(oembed_response))
                .layer(
                    ServiceBuilder::new()
                        .layer(from_fn(add_user_info_to_request))
                )
        )
        .nest(
            "/resource/{resource_id}",
            Router::new()
                .route("/master.m3u8", get(get_video_master_playlist))
                .route("/thumbnail.jpg", get(get_video_thumnail))
                .route("/stream_{index}/{file_in_directory}", get(get_stream_asset))
                .route("/metadata", get(resource_metadata))
                .layer(
                    ServiceBuilder::new()
                        .layer(from_fn(add_user_info_to_request))
            )
        ).layer(ip_source.into_extension());
        
        
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("Failed to bind TCP listener");

    let resource_status_listener_task = resource_status_listener();

    tracing::info!("Listening on port {}", port);
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
    let resource = db::get_active_resource_by_id(&resource_id);
    if let Some(resource) = resource {
        if resource.resource_type == resource_type && has_access_to_resource(&user_info, &resource) {

            if transfer_quota_exceeded() {
                tracing::warn!("Transfer quota exceeded for user {}", user_info.as_ref().map_or("unknown", |u| &u.user_id));
                // Hey, bandwidth is expensive. 
                return StatusCode::PAYMENT_REQUIRED.into_response();
            }

            let object_name = format!("{}/{}/{}", RESOURCE_FOLDER, resource.id, file_in_directory);
            let s3_client = get_s3_client().await;
          

            match get_object_stream(
                s3_client,
                &object_name,
            ).await {
                Ok((stream, file_size, _modified)) => {
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
    object_name: &str,
) -> Result<(ByteStream, i64, chrono::DateTime<chrono::Utc>), DisplayErrorContext<impl std::error::Error>> {
    tracing::info!("Getting object stream for object: {}", object_name);
    let get_object_output = match s3_client
        .get_object()
        .bucket(s3_bucket())
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
        tracing::warn!("Object {} has no content length", object_name);
    }

    let modified = get_object_output
        .last_modified
        .map(|dt| dt.to_chrono_utc())
        .unwrap().unwrap();

    Ok((get_object_output.body, get_object_output.content_length.unwrap_or(0), modified))
}