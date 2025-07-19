mod models;
mod db;

use std::env;
use aws_sdk_sqs::Client;

use tracing_subscriber::filter;


use models::*;


#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_level(true)
        .with_max_level(filter::LevelFilter::INFO)
        .init();


    let queue_url = env::var("AUDIT_EVENT_QUEUE_URL").expect("AUDIT_EVENT_QUEUE_URL not set");
    let client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);

    loop {
       let audit_event_opt = receive_audit_event(&client, &queue_url).await
            .unwrap_or_else(|err| {
                tracing::error!("Error receiving audit event notification: {}", err);
                None
            });

        if let Some(audit_event) = audit_event_opt {

            db::insert_audit_event(
                audit_event.message.user_id.as_deref(),
                &audit_event.message.client_ip,
                audit_event.message.event_type,
                audit_event.message.target.as_deref(),
                audit_event.message.event_details,
                audit_event.timestamp
            ).unwrap_or_else(|err| {
                tracing::error!("Error inserting audit event into database: {}", err);
            });


            delete_message(&client, &queue_url, &audit_event.receipt_handle).await
                .unwrap_or_else(|err| {
                    tracing::error!("Error deleting message: {}", err);
                });
        }


        // Sleep for a while before checking the queue again
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}


async fn receive_audit_event(client: &Client, queue_url: &str) -> Result<Option<AuditEvent>, aws_sdk_sqs::Error> {
    let rcv_message_output = client
        .receive_message()
        .queue_url(queue_url)
        .message_system_attribute_names(aws_sdk_sqs::types::MessageSystemAttributeName::SentTimestamp)
        .max_number_of_messages(1)
        .send()
        .await?;
    
    for message in rcv_message_output.messages.unwrap_or_default() {
        let body = match message.body {
            Some(ref body) => body,
            None => {
                tracing::warn!("Received message with no body, skipping.");
                continue;
            }
        };

        let audit_message: AuditMessage = match serde_json::from_str(body){
            Ok(msg) => msg,
            Err(err) => {
                tracing::error!("Failed to parse message body as JSON: {}", err);
                continue;
            }
        };

        tracing::info!("Message attributes: {:?}", message.attributes());
        let sent_timestamp = message
            .attributes()
           .unwrap()[&aws_sdk_sqs::types::MessageSystemAttributeName::SentTimestamp].clone().parse::<i64>().unwrap();

        return Ok(Some(AuditEvent {
            message: audit_message,
            receipt_handle: message.receipt_handle.unwrap_or_default(),
            timestamp: chrono::DateTime::from_timestamp_millis(sent_timestamp).unwrap(),
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