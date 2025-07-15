use crate::db;

use serde::{Deserialize, Serialize};


pub struct ResourceStatusUpdateEvent {
    pub message: ResourceStatusUpdateMessage,
    pub receipt_handle: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "status")]
pub enum ResourceStatusUpdateMessage {
    #[serde(rename = "uploaded")]
    ResourceUploaded {
        user_id: String,
        object_name: String,
        file_name: String,
    },
    #[serde(rename = "failed")]
    ResourceProcessingFailed {
        object_name: String,
    },
    #[serde(rename = "processing")]
    ResourceProcessingStarted {
        object_name: String,
    },
    #[serde(rename = "type_resolved")]
    ResourceTypeResolved {
        object_name: String,
        resource_type: String,
    },
    #[serde(rename = "processed")]
    ResourceProcessed {
        object_name: String,
    },
}



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



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePublicStatusUpdate {
    pub is_public: bool,
}