
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuditEvent {
    pub message: AuditMessage,
    pub receipt_handle: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
pub struct AuditMessage {
    pub event_type: String,
    pub user_id: Option<String>,
    pub client_ip: String,
    pub target: Option<String>,
    pub event_details: Option<serde_json::Value>, 
}