use std::env;
use std::io;

use aws_sdk_sqs::Client;
use clamav_client;
use tracing_subscriber::filter;
use reqwest;
use serde_json;


use futures_util::stream::StreamExt;
//use futures_util::future::future::FutureExt;
//use futures_util::stream::stream::StreamExt;
//use generic_array::functional::FunctionalSequence;
//use std::iter::Iterator;


#[derive(Debug, serde::Deserialize)]
struct UploadMessage {
    pub presigned_url: String,
}

struct UploadEvent {
    pub message: UploadMessage,
    pub receipt_handle: String,
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


    let queue_url = env::var("UPLOAD_QUEUE_URL").expect("UPLOAD_QUEUE_URL not set");
    let client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);

    loop {
       let upload_event_opt = receive_upload_notification(&client, &queue_url).await
            .unwrap_or_else(|err| {
                tracing::error!("Error receiving upload notification: {}", err);
                None
            });

        if let Some(upload_event) = upload_event_opt {

            tracing::info!("Received presigned URL: {}", upload_event.message.presigned_url);
            if let Ok(_) = scan_file(&upload_event.message.presigned_url).await {
                tracing::info!("File scan completed successfully, no viruses found.");
            } else {
                tracing::warn!("File scan failed or file is infected with a virus.");
            }


            delete_message(&client, &queue_url, &upload_event.receipt_handle)
                .await
                .unwrap_or_else(|err| {
                    tracing::error!("Error deleting message: {}", err);
                });
        }

        // Sleep for a while before checking the queue again
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

}


async fn receive_upload_notification(client: &Client, queue_url: &str) -> Result<Option<UploadEvent>, aws_sdk_sqs::Error> {
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

        let upload_message: UploadMessage = match serde_json::from_str(&body){
            Ok(msg) => msg,
            Err(err) => {
                tracing::error!("Failed to parse message body as JSON: {}", err);
                continue;
            }
        };

        return Ok(Some(UploadEvent {
            message: upload_message,
            receipt_handle: message.receipt_handle.unwrap_or_default(),
        }));         

    }

    Ok(None)
}

/// Scans the file at the given presigned URL using ClamAV.
/// 
/// # Arguments
/// * `presigned_url` - A string slice that holds the presigned URL of the file to be scanned.
/// # Returns
/// * `Result<bool, Box<dyn std::error::Error>>` - If file is fine, returns `Ok(true)`, 
///
async fn scan_file(presigned_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Simulate a virus scan by just logging the URL
    tracing::info!("Scanning file at presigned URL: {}", presigned_url);

    let http_client = reqwest::Client::new(); 
    let clamd_tcp = clamav_client::tokio::Tcp{ host_address: "localhost:3310" };

    let clamd_available =  clamav_client::tokio::ping(clamd_tcp).await;

    match clamd_available {
        Ok(_) => tracing::info!("ClamAV is available"),
        Err(err) => {
            tracing::error!("ClamAV is not available: {}", err);
            return Err(Box::new(err));
        }
    }

    let reqwest_stream = http_client.get(presigned_url)
        .send()
        .await?
        .bytes_stream();


    let stream = reqwest_stream.map(|result| {
    result.map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    });

    
       

    let scan_response = clamav_client::tokio::scan_stream(stream, clamd_tcp, None).await
        .map_err(|err| {
            tracing::error!("Virus scan failed: {}", err);
            Box::new(err) as Box<dyn std::error::Error>
        })?;
    
    let is_file_clean = clamav_client::clean(&scan_response)
        .map_err(|err| {
            tracing::error!("Failed to parse scan result: {}", err);
            Box::new(err) as Box<dyn std::error::Error>
        })?;

    if is_file_clean {
        tracing::info!("File is clean, no viruses found.");
    } else {
        tracing::warn!("File is infected with a virus!");
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "File is infected with a virus",
        )));
    }

    Ok(())
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
