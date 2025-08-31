use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Serialize)] 
#[allow(dead_code)]
pub struct UserQuota {
    pub used_quota: i64,
    pub total_quota: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewChunkUploadRequest {
    pub file_name: String,
    pub file_size: usize,
    pub integrity_check_type: IntegrityCheckType,
    pub integrity_check_value: Option<String>,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegrityCheckType {
    None,
    Crc32,
}

impl IntegrityCheckType {
    pub fn as_str(&self) -> &str {
        match self {
            IntegrityCheckType::None => "none",
            IntegrityCheckType::Crc32 => "crc32",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NewChunkUploadResponse {
    pub upload_id: String,
    pub chunk_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompleteUploadRequest {
    pub upload_id: String,
}
