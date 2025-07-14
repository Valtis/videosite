use crate::db;

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub is_public: bool,
    pub resource_name: String,
    pub resource_type: String,
    pub resource_status: String,
    pub created_at: String,
}

impl From<db::Resource> for Resource {
    fn from(resource: db::Resource) -> Self {
        Resource {
            id: resource.id.to_string(),
            is_public: resource.is_public,
            resource_name: resource.resource_name,
            resource_type: resource.resource_type,
            resource_status: resource.resource_status,
            created_at: resource.created_at.to_rfc3339(),
        }
    }
}
