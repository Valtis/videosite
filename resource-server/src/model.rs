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
        metadata: ProducedResourceMetadata,
    },
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)] // Audio, Image variants not used yet
pub enum ProducedResourceMetadata {
    Video(Vec<VideoData>),
    Audio(AudioData),
    Image(ImageData),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Not used yet
pub struct AudioData {
    pub duration: f64, 
    pub bitrate: u32, 
    pub sample_rate: u32, // Sample rate in Hz
}

#[derive(Debug, Deserialize)]
pub struct VideoData {
    pub duration: f64, 
    pub width: u32,   
    pub height: u32,  
    pub bitrate: u32, 
    pub frame_rate: f64, 
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Not used yet
pub struct ImageData {
    pub width: u32,   // Width in pixels
    pub height: u32,  // Height in pixels
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


#[derive(Debug, Clone, Serialize)]
pub struct OEmbedResponse {
    pub version: String,
    pub title: String,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub provider_name: Option<String>,
    pub provider_url: Option<String>,
    pub cache_age: Option<u32>,
    pub thumbnail_url: Option<String>,
    pub thumbnail_width: Option<u32>,
    pub thumbnail_height: Option<u32>,
    #[serde(flatten)]
    pub resource: OEmbedResourceType,
    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OEmbedResourceType {
    Photo { url: String, width: i32, height: i32 },
    Video { html: String, width: i32, height: i32 },
}



#[derive(Debug, Clone,  Serialize)]
pub struct ResourceMetadataResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(flatten)]
    pub resource_metadata: ResourceMetadata,
}

#[derive(Debug, Clone,  Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ResourceMetadata {
    Video{ width: i32, height: i32, duration_seconds: i32, bit_rate: i32, frame_rate: f32 },
}