use std::env;

use axum::{
    extract::{
        DefaultBodyLimit, Multipart
    }, http::StatusCode, middleware::from_fn, response::IntoResponse, routing::{get, post}, Extension, Router
};

use uuid;

use aws_sdk_s3 as s3;
use s3::presigning::PresigningConfig;
use s3::primitives::ByteStream;
use s3::types::CompletedMultipartUpload;

use tower::ServiceBuilder;

use auth_check::{auth_middleware, UserInfo};

use tracing_subscriber::filter;

const UPLOAD_BUCKET: &str = "upload"; 

#[axum::debug_handler]
async fn upload_handler(user_info: Extension<UserInfo>, mut multipart: Multipart) -> impl IntoResponse {


    while let Some(field) = multipart.next_field().await.unwrap() {
        let (presigned_uri, object_name, file_name, file_size) = upload_file(field).await;
        tracing::info!("File uploaded successfully, presigned URL: {}", presigned_uri);
        queue_upload_event(&user_info, presigned_uri, &object_name, &file_name, file_size).await;
    }

    return(StatusCode::OK, "upload handler reached").into_response();
}

async fn upload_file(mut field: axum::extract::multipart::Field<'_>) -> (String, String, String, usize) {

    let name = field.name().unwrap_or("not set").to_string();
    let content_type = field.content_type().map(|ct| ct.to_string());
    let filename = field.file_name().map(|fnm| fnm.to_string());
    
    tracing::info!("Received field: name={}, content_type={:?}, filename={:?}", name, content_type, filename);
    

    let config = aws_config::load_from_env().await;
    let client = s3::Client::new(&config);

    let client = if let Ok(var) = env::var("USE_PATH_STYLE_BUCKETS") {
        if var.to_lowercase() == "true" {
            tracing::info!("Using path-style buckets");
            let config_builder = client.config().clone().to_builder();
            s3::Client::from_conf(config_builder.force_path_style(true).build())
        } else {
            client
        }
    } else {
        client
    };

    let object_name = uuid::Uuid::new_v4().to_string();
    tracing::info!("Uploading file to S3 with object name: {}", object_name);

    let multi_part_upload = client.create_multipart_upload()
        .bucket(UPLOAD_BUCKET) 
        .key(object_name.clone())
        .send()
        .await
        .expect("Failed to create multipart upload");

    let upload_id = multi_part_upload.upload_id().expect("Upload ID not found");
    tracing::info!("Created multipart upload with ID: {}", upload_id);

    let mut part_number = 1;
    let mut file_size = 0;
    let mut completed_parts = Vec::new();
    let mut buffer = Vec::new();
    while let Some(chunk) = field.chunk().await.expect("Failed to read chunk") {
        file_size += chunk.len();
        buffer.extend_from_slice(&chunk);

        if buffer.len() > 5 * 1024 * 1024 { 
            upload_chunk(&client, buffer, &object_name, upload_id, part_number, &mut completed_parts).await;
            buffer = Vec::new(); // Reset buffer after uploading
            part_number += 1;
        }
    }

    if !buffer.is_empty() {
        upload_chunk(&client, buffer, &object_name, upload_id, part_number, &mut completed_parts).await;
    }


    tracing::info!("Completing multipart upload for object: {}", object_name);

    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    client.complete_multipart_upload()
        .bucket(UPLOAD_BUCKET)
        .key(&object_name)
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await
        .expect("Failed to complete multipart upload");

    (client.get_object()
        .bucket(UPLOAD_BUCKET)
        .key(&object_name)
        .presigned(
            PresigningConfig::builder()
                .expires_in(std::time::Duration::from_secs(3600*7)) // 7 hours, this could be a video and processing can take a while
                .build()
                .expect("Failed to build presigning config")
        ).await
        .expect("Failed to generate presigned URL")
        .uri().to_string(),
        object_name.clone(),
        filename.unwrap_or(object_name),
        file_size
    )
}

async fn upload_chunk(
    client: &s3::Client,
    buffer: Vec<u8>,
    object_name: &str,
    upload_id: &str,
    part_number: i32,
    completed_parts: &mut Vec<s3::types::CompletedPart>
) {
    let bytes = ByteStream::from(buffer);
    let part = client.upload_part()
        .bucket(UPLOAD_BUCKET) 
        .key(object_name)
        .part_number(part_number)
        .upload_id(upload_id)
        .body(bytes.into())
        .send()
        .await
        .expect("Failed to upload part");

    completed_parts.push(s3::types::CompletedPart::builder()
        .part_number(part_number)
        .e_tag(part.e_tag().unwrap_or("not set").to_string())
        .build());
}

async fn queue_upload_event(user_info: &UserInfo, presigned_uri: String, object_name: &str, file_name: &str, file_size: usize) {
    let sqs_client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);
    let upload_queue_url = env::var("UPLOAD_QUEUE_URL").expect("UPLOAD_QUEUE_URL not set");
    let resource_status_queue_url = env::var("RESOURCE_STATUS_QUEUE_URL").expect("RESOURCE_STATUS_QUEUE_URL not set");


    let upload_json_msg = serde_json::json!({
        "presigned_url": presigned_uri,
        "file_size": file_size,
        "object_name": object_name,
    }).to_string(); 

    let resource_status_json_msg = serde_json::json!({
        "user_id": user_info.user_id,
        "object_name": object_name,
        "file_name": file_name,
        "status": "uploaded",
        "origin_file_path": format!("/{}/{}", UPLOAD_BUCKET, object_name),
    }).to_string();

    tracing::info!("Sending message {} to SQS queue: {}", upload_json_msg, upload_queue_url);

    sqs_client.send_message()
        .queue_url(upload_queue_url)
        .message_body(upload_json_msg)
        .send()
        .await
        .expect("Failed to send upload message to SQS");

    tracing::info!("Sending resource status message {} to SQS queue: {}", resource_status_json_msg, resource_status_queue_url);
    sqs_client.send_message()
        .queue_url(resource_status_queue_url)
        .message_body(resource_status_json_msg)
        .send()
        .await
        .expect("Failed to send resource status message to SQS");


}

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .pretty()
        .with_max_level(filter::LevelFilter::INFO)
        .init();

    let app = Router::new()
        .route("/upload/health", get(|| async { "ok" }))
        .route("/upload/file", post(upload_handler))
            .layer(
                ServiceBuilder::new()
                    .layer(DefaultBodyLimit::max(4096*1024*1024)) // 4gb limit
                    .layer(from_fn(auth_middleware))
            );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await
        .expect("failed to bind tcp listener");

    axum::serve(listener, app)
        .await
        .expect("failed to start server");
}