use std::env;
use aws_sdk_sqs::Client;


#[derive(Debug, serde::Serialize)]
pub struct AuditEvent<'a> {
    pub event_type: String,
    pub user_id: Option<&'a str>,
    pub client_ip: &'a str,
    pub target: Option<&'a str>,
    pub event_details: Option<serde_json::Value>,
}

pub async fn send_audit_event<'a>(event: AuditEvent<'a>) -> Result<(), Box<dyn std::error::Error>> {
    let audit_event_queue_url = env::var("AUDIT_EVENT_QUEUE_URL")
        .expect("AUDIT_EVENT_QUEUE_URL not set");

    let serialized_event = serde_json::to_string(&event)?;
    let client = Client::new(&aws_config::load_from_env().await);
    client
        .send_message()
        .queue_url(&audit_event_queue_url)
        .message_body(serialized_event)
        .send()
        .await?;


    Ok(())
}