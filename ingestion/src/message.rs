use crate::{get_object_path, UserInfo};

use std::env;

pub async fn queue_upload_event(user_info: &UserInfo, presigned_uri: String, object_name: &str, file_name: &str, file_size: usize) {
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
        "origin_file_path": get_object_path(object_name), 
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