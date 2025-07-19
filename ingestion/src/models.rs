use serde::Serialize;

#[derive(Debug, Clone, Serialize)] 
#[allow(dead_code)]
pub struct UserQuota {
    pub used_quota: i64,
    pub total_quota: i64,
}