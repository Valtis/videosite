use serde::Serialize;

#[derive(Debug, Clone, Serialize)] 
pub struct UserQuota {
    pub used_quota: i64,
    pub total_quota: i64,
}