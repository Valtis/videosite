use crate::{get_object_path, s3_bucket};

use std::env;

use aws_sdk_s3::{self as s3};
use s3::presigning::PresigningConfig;
use s3::primitives::ByteStream;

use s3::types::{CompletedMultipartUpload, CompletedPart};
use s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use aws_sdk_s3::operation::complete_multipart_upload::CompleteMultipartUploadOutput;


pub async fn get_s3_client() -> s3::Client {
    let config = aws_config::load_from_env().await;
    let client = s3::Client::new(&config);

    if let Ok(var) = env::var("USE_PATH_STYLE_BUCKETS") {
        if var.to_lowercase() == "true" {
            tracing::info!("Using path-style buckets");
            let config_builder = client.config().clone().to_builder();
            s3::Client::from_conf(config_builder.force_path_style(true).build())
        } else {
            client
        }
    } else {
        client
    }
}

/// upload a full file coming from request multipart form, as S3 multipart upload
pub async fn upload_file(mut field: axum::extract::multipart::Field<'_>) -> (String, String, String, usize) {

    let name = field.name().unwrap_or("not set").to_string();
    let content_type = field.content_type().map(|ct| ct.to_string());
    let filename = field.file_name().map(|fnm| fnm.to_string());
    
    tracing::info!("Received field: name={}, content_type={:?}, filename={:?}", name, content_type, filename);
    

    let client = get_s3_client().await;
    

    let object_name = uuid::Uuid::new_v4().to_string();
    tracing::info!("Uploading file to S3 with object name: {}", object_name);

    let multi_part_upload = initiate_multipart_upload(&client, &object_name).await;


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
            let completed_part = upload_chunk(&client, buffer, &object_name, upload_id, part_number).await;
            completed_parts.push(completed_part);
            buffer = Vec::new(); // Reset buffer after uploading
            part_number += 1;
        }
    }

    if !buffer.is_empty() {
        let completed_part = upload_chunk(&client, buffer, &object_name, upload_id, part_number).await;
        completed_parts.push(completed_part);
    }


    tracing::info!("Completing multipart upload for object: {}", object_name);

    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    complete_chunk_upload(
        &client, 
        &object_name, 
        upload_id, 
        completed_multipart_upload).await;

    let expires_in_seconds = 7 * 60 * 60; // 7 hours
    (
        create_presigned_url(&client, &object_name, expires_in_seconds).await,
        object_name.clone(),
        filename.unwrap_or(object_name),
        file_size
    )

    
}

pub async fn initiate_multipart_upload(client: &s3::Client, object_name: &str) -> CreateMultipartUploadOutput {
    return client.create_multipart_upload()
        .bucket(s3_bucket())
        .key(get_object_path(object_name))
        .send()
        .await
        .expect("Failed to initiate multipart upload")
}



pub async fn upload_chunk(
    client: &s3::Client,
    buffer: Vec<u8>,
    object_name: &str,
    upload_id: &str,
    part_number: i32,
) -> CompletedPart {
    let bytes = ByteStream::from(buffer);
    let part = client.upload_part()
        .bucket(s3_bucket()) 
        .key(get_object_path(object_name))
        .part_number(part_number)
        .upload_id(upload_id)
        .body(bytes.into())
        .send()
        .await
        .expect("Failed to upload part");

    return CompletedPart::builder()
        .part_number(part_number)
        .e_tag(part.e_tag().unwrap_or("not set").to_string())
        .build();
}

pub async fn complete_chunk_upload(client: &s3::Client, object_name: &str, upload_id: &str, completed_multipart_upload: CompletedMultipartUpload) -> CompleteMultipartUploadOutput {
    client.complete_multipart_upload()
        .bucket(s3_bucket())
        .key(get_object_path(object_name))
        .upload_id(upload_id)
        .multipart_upload(completed_multipart_upload)
        .send()
        .await
        .expect("Failed to complete multipart upload")
}

pub async fn abort_chunk_upload(client: &s3::Client, object_name: &str, upload_id: &str) {
    client.abort_multipart_upload()
        .bucket(s3_bucket())
        .key(get_object_path(object_name))
        .upload_id(upload_id)
        .send()
        .await
        .expect("Failed to abort multipart upload");
}



pub async fn delete_file(object_name: &str) {
    let client = get_s3_client().await;
    client.delete_object()
        .bucket(s3_bucket())
        .key(get_object_path(object_name))
        .send()
        .await
        .expect("Failed to delete file from S3");
}

pub async fn create_presigned_url(client: &s3::Client, object_name: &str, expires_in_seconds: u64) -> String {
    client.get_object()
        .bucket(s3_bucket())
        .key(get_object_path(&object_name))
        .presigned(
            PresigningConfig::builder()
                .expires_in(std::time::Duration::from_secs(expires_in_seconds)) // 7 hours, this could be a video and processing can take a while
                .build()
                .expect("Failed to build presigning config")
        ).await
        .expect("Failed to generate presigned URL")
        .uri().to_string()
}