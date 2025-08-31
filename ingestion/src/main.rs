mod db;
mod message;
mod models;
mod upload;


use std::{env, f64::consts::E};

use axum::{
    extract::{
        DefaultBodyLimit, Multipart,
    }, http::StatusCode, middleware::from_fn, response::{IntoResponse, Redirect}, routing::{get, post}, Extension, Json, Router
};

use axum_client_ip::{ClientIpSource, ClientIp};


use uuid;
use tower::ServiceBuilder;

use auth_check::{auth_middleware, UserInfo};
use audit::{send_audit_event, AuditEvent};

use tracing_subscriber::filter;

use message::*;
use upload::*;


const UPLOAD_FOLDER: &str = "upload"; 

fn get_object_path(object_name: &str) -> String {
    format!("{}/{}", UPLOAD_FOLDER, object_name)
}

fn s3_bucket() -> String {
    env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set")
}

#[axum::debug_handler]
async fn upload_handler(ClientIp(client_ip): ClientIp, user_info: Extension<UserInfo>, mut multipart: Multipart) -> Redirect {
    tracing::info!("Starting file upload handler for user: {}", user_info.user_id);
    let user_total_quota = db::user_quota(&user_info.user_id);
    let mut used_quota = db::used_user_quota(&user_info.user_id);

    while let Some(field) = multipart.next_field().await.unwrap() {

        let (presigned_uri, object_name, file_name, file_size) = upload_file(field).await;

        used_quota += file_size as i64;
        if used_quota > user_total_quota {
            tracing::error!("User {} has exceeded their upload quota. Used: {}, Total: {}", user_info.user_id, used_quota, user_total_quota);
            // since we cannot get the size before storing the file, we need to delete the file from S3
            delete_file(&object_name).await;
            tracing::error!("File {} deleted from S3 due to quota exceeded", object_name);

            send_audit_event(AuditEvent {
                event_type: "file_upload".to_string(),
                user_id: Some(&user_info.user_id),
                client_ip: &client_ip.to_string(),
                target: Some(&object_name),
                event_details: Some(serde_json::json!({
                    "file_name": file_name,
                    "file_size": file_size,
                    "error": "quota_exceeded"
                })),
            }).await.unwrap_or_else(|e| {
                tracing::error!("Failed to send audit event: {}", e);
            });

            return Redirect::to("/index.html?error=quota_exceeded");
        }

        db::insert_new_upload(
            &user_info.user_id,
            &object_name,
            file_size as i64,
        );
        tracing::info!("File uploaded successfully, presigned URL: {}", presigned_uri);
        queue_upload_event(&user_info, presigned_uri, &object_name, &file_name, file_size).await;

        send_audit_event(AuditEvent {
            event_type: "file_upload".to_string(),
            user_id: Some(&user_info.user_id),
            client_ip: &client_ip.to_string(),
            target: Some(&object_name),
            event_details: Some(serde_json::json!({
                "file_name": file_name,
                "file_size": file_size,
            })),
        }).await.unwrap_or_else(|e| {
            tracing::error!("Failed to send audit event: {}", e);
        });
    }

    Redirect::to("/index.html")
}

#[axum::debug_handler]
async fn user_quota(user_info: Extension<UserInfo>) -> impl IntoResponse {
    let total_quota = db::user_quota(&user_info.user_id);
    let used_quota = db::used_user_quota(&user_info.user_id);

    let response = models::UserQuota {
        used_quota,
        total_quota,
    };

    (StatusCode::OK, axum::Json(response))
}



/// init new chunk upload, returns the ID and chunk size for uploading the individual chunks
/// the client will need to provide this ID when uploading chunks
 
#[axum::debug_handler]
async fn init_chunk_upload(
    ClientIp(client_ip): ClientIp,
    user_info: Extension<UserInfo>,
    Json(payload): Json<models::NewChunkUploadRequest>,
) -> impl IntoResponse {
    let user_quota = db::user_quota(&user_info.user_id);
    let used_quota = db::used_user_quota(&user_info.user_id);
    if used_quota + payload.file_size as i64 > user_quota {
        tracing::error!("User {} has exceeded their upload quota. Used: {}, Total: {}", user_info.user_id, used_quota, user_quota);
        let error_response = models::ErrorResponse {
                     error: "quota_exceeded".to_owned() 
        };
        // this is still on the normal path, so we do not audit log this; user did not manage to start an upload
        return (StatusCode::PAYMENT_REQUIRED, axum::Json(error_response)).into_response();
    }


    let object_name = uuid::Uuid::new_v4().to_string();
    let chunk_size = env::var("CHUNK_SIZE")
        .unwrap_or_else(|_| "5242880".to_string()) // default to 5MB
        .parse::<usize>()
        .expect("CHUNK_SIZE must be a valid number");
    if chunk_size < 5 * 1024 * 1024 {
        tracing::error!("CHUNK_SIZE must be at least 5MB");
        let error_response = models::ErrorResponse {
                     error: "Internal server error".to_owned() 
        };
        return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(error_response)).into_response();
    }

    let client = get_s3_client().await;
    let upload = initiate_multipart_upload(&client, &object_name).await;
    let aws_upload_id = upload.upload_id().expect("Upload ID not found");
        
    let response = models::NewChunkUploadResponse {
        upload_id: object_name.clone(),
        chunk_size,
    };
    
    db::init_chunk_upload(
        &object_name, 
        &aws_upload_id,
        &user_info.user_id,
        &payload.file_name,
        &payload.integrity_check_type.as_str(),
        payload.integrity_check_value.as_deref(),
        chunk_size as i64,
    );

    send_audit_event(AuditEvent {
        event_type: "init_chunk_upload".to_string(),
        user_id: Some(&user_info.user_id),
        client_ip: &client_ip.to_string(),
        target: Some(&object_name),
        event_details: Some(serde_json::json!({
            "file_name": payload.file_name,
            "file_size": payload.file_size,
            "integrity_check_type": payload.integrity_check_type.as_str(),
            "integrity_check_value": payload.integrity_check_value,
            "chunk_size": chunk_size,
        })),
    }).await.unwrap_or_else(|e| {
        tracing::error!("Failed to send audit event: {}", e);
    });

    (StatusCode::OK, axum::Json(response)).into_response()
}

#[axum::debug_handler]
async fn chunk_upload(
    user_info: Extension<UserInfo>,
    ClientIp(client_ip): ClientIp,
    query_params: axum::extract::Query<std::collections::HashMap<String, String>>,
    mut multipart: Multipart
) -> impl IntoResponse {

    tracing::info!("DEBUG query params: {:?}", query_params);
    let upload_id = query_params.get("upload_id");
    let chunk_index = query_params.get("chunk_index");
    if upload_id.is_none() || chunk_index.is_none() {
        tracing::error!("Missing upload_id or chunk_index in query parameters");
        return StatusCode::BAD_REQUEST;
    }

    let upload_id = upload_id.unwrap();
    let chunk_index: usize = match chunk_index.unwrap().parse() {
        Ok(idx) => idx,
        Err(_) => {
            tracing::error!("Invalid chunk_index: {}", chunk_index.unwrap());
            return StatusCode::BAD_REQUEST;
        }
    };

    let chunk_upload = db::get_active_chunk_upload(&user_info.user_id, &upload_id);
    if chunk_upload.is_none() {
        tracing::error!("No active chunk upload found for user {} and upload ID {}", user_info.user_id, upload_id);
        return StatusCode::NOT_FOUND;
    }


    let mut buffer = vec![];
    let mut file_size = 0;
    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name().unwrap_or("") == "file" {

            // Read all bytes at once instead of chunking
            match field.bytes().await {
                Ok(bytes) => {
                    file_size = bytes.len();
                    buffer = bytes.to_vec();
                }
                Err(e) => {
                    tracing::error!("Failed to read field bytes: {}", e);
                    return StatusCode::BAD_REQUEST;
                }
            }
        } else {
            tracing::warn!("Unexpected field: {}", field.name().unwrap_or("unknown"));
        }
    }




    let chunk_upload = chunk_upload.unwrap();
    let user_quota = db::user_quota(&user_info.user_id);
    let used_quota = db::used_user_quota(&user_info.user_id);
    if used_quota + file_size as i64 > user_quota {
        tracing::error!("User {} has exceeded their upload quota. Used: {}, Total: {}", user_info.user_id, used_quota, user_quota);

        // the initial quota check must have passed, so this is bit weird (could be just two parallel uploads). Regardless,
        // let's audit log it as this may cause at least people to ask what's going on
        send_audit_event(AuditEvent {
            event_type: "chunk_upload".to_string(),
            user_id: Some(&user_info.user_id),
            client_ip: &client_ip.to_string(),
            target: Some(&upload_id),
            event_details: Some(serde_json::json!({
                "chunk_index": chunk_index,
                "chunk_size": file_size,
                "error": "quota_exceeded"
            })),
        }).await.unwrap_or_else(|e| {
            tracing::error!("Failed to send audit event: {}", e);
        });
        
        // cancel the upload in S3 and delete the file
        // TODO let's dryrun first without S3
        let client = get_s3_client().await;
        abort_chunk_upload(&client, &chunk_upload.object_name.to_string(),  &chunk_upload.aws_upload_id).await;
        db::delete_chunks_for_upload(&user_info.user_id, &chunk_upload.object_name.to_string());
        db::delete_chunk_upload_record(&user_info.user_id, &chunk_upload.object_name.to_string());

        return StatusCode::PAYMENT_REQUIRED;
    }

    if file_size > chunk_upload.chunk_size as usize {
        tracing::error!("Chunk size {} exceeds the allowed chunk size {}", file_size, chunk_upload.chunk_size);
        return StatusCode::BAD_REQUEST;
    } else if file_size == 0 {
        tracing::error!("Chunk size is zero");
        return StatusCode::BAD_REQUEST;
    }

    let client = get_s3_client().await;
    let completed_part = upload_chunk(
        &client, 
        buffer, 
        &chunk_upload.object_name.to_string(), 
        &chunk_upload.aws_upload_id, 
        chunk_index as i32).await;
    
    db::save_uploaded_chunk_information(
        &user_info.user_id, 
        &upload_id, 
        &completed_part.e_tag().expect("ETag not found"),
        completed_part.part_number().expect("Part number not found") as usize,
    );

    db::update_received_bytes_for_chunk_upload(
        &user_info.user_id,
        &upload_id,
        file_size as i64,
    );
    

    send_audit_event(
        AuditEvent {
            event_type: "chunk_upload".to_string(),
            user_id: Some(&user_info.user_id),
            client_ip: &client_ip.to_string(),
            target: Some(&chunk_upload.aws_upload_id),
            event_details: Some(serde_json::json!({
                "chunk_index": chunk_index,
                "chunk_size": file_size,
            })),
        }
    ).await.unwrap_or_else(|e| {
        tracing::error!("Failed to send audit event: {}", e);
    });
    
    StatusCode::NO_CONTENT

}

#[axum::debug_handler]
async fn complete_chunk_upload_handler(
    ClientIp(client_ip): ClientIp,
    user_info: Extension<UserInfo>,
    Json(payload): Json<models::CompleteUploadRequest>,
) -> impl IntoResponse {
    let active_upload = db::get_active_chunk_upload(&user_info.user_id, &payload.upload_id);
    if active_upload.is_none() {
        tracing::error!("No active chunk upload found for user {} and upload ID {}", user_info.user_id, payload.upload_id);
        return StatusCode::NOT_FOUND;
    }

    let active_upload = active_upload.unwrap();
    let user_quota = db::user_quota(&user_info.user_id);
    let used_quota = db::used_user_quota(&user_info.user_id);

    // uploaded chunks are included in the used quota, do not add the file size from active_upload
    // or we double-count it
    if used_quota > user_quota {
        tracing::error!("User {} has exceeded their upload quota. Used: {}, Total: {}", user_info.user_id, used_quota, user_quota);

        send_audit_event(AuditEvent {
            event_type: "complete_chunk_upload".to_string(),
            user_id: Some(&user_info.user_id),
            client_ip: &client_ip.to_string(),
            target: Some(&payload.upload_id),
            event_details: Some(serde_json::json!({
                "error": "quota_exceeded"
            })),
        }).await.unwrap_or_else(|e| {
            tracing::error!("Failed to send audit event: {}", e);
        });

        let client = get_s3_client().await;
        abort_chunk_upload(&client, &active_upload.object_name.to_string(),  &active_upload.aws_upload_id).await;
        db::delete_chunks_for_upload(&user_info.user_id, &active_upload.object_name.to_string());
        db::delete_chunk_upload_record(&user_info.user_id, &active_upload.object_name.to_string());
        return StatusCode::PAYMENT_REQUIRED;
    }


    let uploaded_parts = db::get_uploaded_parts(&user_info.user_id, &payload.upload_id);
    if uploaded_parts.is_empty() {
        tracing::error!("No uploaded parts found for user {} and upload ID {}", user_info.user_id, payload.upload_id);
        return StatusCode::BAD_REQUEST;
    }

    let mut completed_parts: Vec<aws_sdk_s3::types::CompletedPart> = uploaded_parts.iter().map(|part| {
        aws_sdk_s3::types::CompletedPart::builder()
            .part_number(part.part_number as i32)
            .e_tag(part.e_tag.clone())
            .build()
    }).collect();

    let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    let client = get_s3_client().await;
    let result = complete_chunk_upload(
        &client, 
        &active_upload.object_name.to_string(),  
        &active_upload.aws_upload_id, 
        completed_multipart_upload).await;

    match active_upload.file_integrity_algorithm.as_str() {
        "crc32" => {
            if let Some(checksum) = result.checksum_crc32() {
                if let Some(expected) = &active_upload.file_integrity_hash {
                    if checksum != expected {
                        tracing::error!("CRC32 checksum mismatch for upload {}: expected {}, got {}", payload.upload_id, expected, checksum);
                        return StatusCode::BAD_REQUEST;
                    }
                } else {
                    tracing::warn!("No expected CRC32 checksum provided for upload {}", payload.upload_id);
                }
            } else {
                tracing::warn!("No CRC32 checksum returned by S3 for upload {}", payload.upload_id);
            }

        },
        "none" => {},
        _ => {
            tracing::warn!("Unknown integrity check algorithm: {}", active_upload.file_integrity_algorithm);
        }
    }


    db::complete_chunk_upload(&user_info.user_id, &payload.upload_id);
    let client = get_s3_client().await;
    let expires_in_seconds = 7 * 60 * 60; // 7 hours
    let presigned_uri = create_presigned_url(&client, &active_upload.object_name.to_string(), expires_in_seconds).await;

    queue_upload_event(
        &user_info, 
        presigned_uri,
        &active_upload.object_name.to_string(), 
        &active_upload.file_name, 
        active_upload.received_bytes as usize
    ).await;

    send_audit_event(AuditEvent {
        event_type: "complete_chunk_upload".to_string(),
        user_id: Some(&user_info.user_id),
        client_ip: &client_ip.to_string(),
        target: Some(&payload.upload_id),
        event_details: Some(serde_json::json!({
            "file_name": active_upload.file_name,
            "file_size": active_upload.received_bytes,
        })),
    }).await.unwrap_or_else(|e| {
        tracing::error!("Failed to send audit event: {}", e);
    });



    StatusCode::NO_CONTENT
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
        .route("/upload/health", get(|| async { "ok" }))
        .nest(
            "/upload",
            Router::new()
            .route("/file", post(upload_handler))
            .route("/chunk", post(chunk_upload))
            .layer(
                ServiceBuilder::new()
                    .layer(DefaultBodyLimit::max(30*1024*1024))  // 30MB max per chunk
                    .layer(from_fn(auth_middleware))
            )
        )
        .nest(
            "/upload",
            Router::new()
            .route("/quota", get(user_quota))
            .route("/init_chunk_upload", post(init_chunk_upload))
            .route("/complete_chunk_upload", post(complete_chunk_upload_handler))

            .layer(
                ServiceBuilder::new()
                    .layer(from_fn(auth_middleware))
            )
        )
        .layer(ip_source.into_extension());
    
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("failed to bind tcp listener");

    axum::serve(listener, app)
        .await
        .expect("failed to start server");
}

