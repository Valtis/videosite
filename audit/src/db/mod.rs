mod schema;

use diesel::prelude::*;
use uuid::Uuid;
use serde_json::Value;

use schema::*;


// provided by the active resources view, which filters out deleted resources
#[derive(Debug, Insertable)]
#[diesel(table_name = audit_event)]
#[allow(dead_code)]
pub struct InsertAuditEvent {
    pub user_id: Option<Uuid>,
    pub client_ip: String,
    pub event_action: String,
    pub action_target: Option<Uuid>,
    pub additional_info: Option<Value>,
    pub event_timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn insert_audit_event(
    user_id: Option<&str>,
    client_ip: &str,
    event_action: String,
    action_target: Option<&str>,
    additional_info: Option<serde_json::Value>,
    event_timestamp: chrono::DateTime<chrono::Utc>
) -> Result<(), diesel::result::Error> {


    let mut conn = get_connection();

    let new_event = InsertAuditEvent {
        user_id: user_id.map(|s| Uuid::parse_str(s).ok()).flatten(),
        client_ip: client_ip.to_string(),
        event_action,
        action_target: action_target.map(|s| Uuid::parse_str(s).ok()).flatten(),
        additional_info: additional_info.map(|mut v| {
            sanitize_json(&mut v);
            v
        }),
        event_timestamp,
    };

    diesel::insert_into(audit_event::table)
        .values(&new_event)
        .execute(&mut conn)?;

    Ok(())
}


fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

// null bytes are not allowed in PostgreSQL jsonb fields. Sanitize the JSON value by removing null bytes.
fn sanitize_json(value: &mut Value) {
    match value {
        Value::String(s) => {
            // Remove all null bytes
            *s = s.replace('\0', "");
        }
        Value::Array(arr) => {
            for v in arr {
                sanitize_json(v);
            }
        }
        Value::Object(map) => {
            for v in map.values_mut() {
                sanitize_json(v);
            }
        }
        _ => {} // Numbers, booleans, null — no action needed
    }
}