mod schema;

use diesel::prelude::*;
use uuid::Uuid;

use bigdecimal::{BigDecimal, ToPrimitive};

use schema::*;

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = active_uploads)]

#[allow(dead_code)]
pub struct UserUpload {
    pub user_id: Uuid,
    pub resource_id: Uuid,
    pub file_size: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = user_uploads)]
#[allow(dead_code)]
pub struct NewUserUpload {
    pub user_id: Uuid,
    pub resource_id: Uuid,
    pub file_size: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = chunk_upload)]
pub struct NewChunkUpload {
    pub object_name: Uuid,
    pub aws_upload_id: String,
    pub user_id: Uuid,
    pub file_name: String,
    pub file_integrity_algorithm: String,
    pub file_integrity_hash: Option<String>,
    pub received_bytes: i64,
    pub chunk_size: i64,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = active_chunk_upload)]
pub struct ActiveChunkUpload {
    pub object_name: Uuid,
    pub aws_upload_id: String,
    pub user_id: Uuid,
    pub chunk_size: i64,
    pub received_bytes: i64,
    pub file_name: String,
    pub file_integrity_algorithm: String,
    pub file_integrity_hash: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = chunk_information)]
pub struct ChunkInformation {
    pub object_name: Uuid,
    pub part_number: i32,
    pub e_tag: String,
    pub user_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = chunk_information)]
pub struct NewChunkInformation {
    pub object_name: Uuid,
    pub part_number: i32,
    pub e_tag: String,
    pub user_id: Uuid,
}


pub fn user_quota(user_id: &str) -> i64 {
    let mut conn = get_connection();

    user_quota::table
        .filter(user_quota::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .select(user_quota::upload_quota)
        .first(&mut conn)
        .optional()
        .expect("Error fetching user quota")
        .unwrap_or(0)
}

pub fn used_user_quota(user_id: &str) -> i64 {
    // sum of:
    //     sum of received_bytes from chunk_upload where user_id is the
    //     sum of file_size from user_uploads where user_id is the provided one
    let mut conn = get_connection();
    let chunk_upload_sum: BigDecimal = chunk_upload::table
        .filter(chunk_upload::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .select(diesel::dsl::sum(chunk_upload::received_bytes))
        .first(&mut conn)
        .optional()
        .expect("Error fetching used quota from chunk_upload")
        .unwrap_or(Some(BigDecimal::from(0)))
        .unwrap_or(BigDecimal::from(0));


    let user_uploads_sum: BigDecimal = user_uploads::table
        .filter(user_uploads::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .select(diesel::dsl::sum(user_uploads::file_size))
        .first(&mut conn)
        .optional()
        .expect("Error fetching used quota from user_uploads")
        .unwrap_or(Some(BigDecimal::from(0)))
        .unwrap_or(BigDecimal::from(0));
    
    if chunk_upload_sum.to_i64().is_none() || user_uploads_sum.to_i64().is_none() {
        // overflow - return max i64. This is unlikely to happen in practice as 
        // I will be broke from the AWS bills way before that happens
        return i64::MAX;
    }

    chunk_upload_sum.to_i64().unwrap().checked_add(user_uploads_sum.to_i64().unwrap()).unwrap_or(i64::MAX)
}


// upload of a single file in one request
pub fn insert_new_upload(user_id: &str, resource_id: &str, file_size: i64) {
    let mut conn = get_connection();

    let new_upload = NewUserUpload {
        user_id: Uuid::parse_str(user_id).unwrap(),
        resource_id: Uuid::parse_str(resource_id).unwrap(),
        file_size,
    };

    diesel::insert_into(user_uploads::table)
        .values(&new_upload)
        .execute(&mut conn)
        .expect("Error inserting new upload");
}

pub fn init_chunk_upload(
    object_name: &str,
    aws_upload_id: &str,
    user_id: &str,
    file_name: &str,
    integrity_check_type: &str,
    integrity_check_value: Option<&str>,
    chunk_size: i64,
) {
    let chunk_upload = NewChunkUpload {
        object_name: Uuid::parse_str(object_name).unwrap(),
        aws_upload_id: aws_upload_id.to_string(),
        user_id: Uuid::parse_str(user_id).unwrap(),
        file_name: file_name.to_string(),
        file_integrity_algorithm: integrity_check_type.to_string(),
        file_integrity_hash: integrity_check_value.map(|s| s.to_string()),
        received_bytes: 0,
        chunk_size,
    };

    let mut conn = get_connection();
    diesel::insert_into(chunk_upload::table)
        .values(&chunk_upload)
        .execute(&mut conn)
        .expect("Error inserting new chunk upload");
}

pub fn update_received_bytes_for_chunk_upload(user_id: &str, object_name: &str, received_bytes: i64) {
    let mut conn = get_connection();

    diesel::update(chunk_upload::table)
        .filter(chunk_upload::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .filter(chunk_upload::object_name.eq(Uuid::parse_str(object_name).unwrap()))
        .set(chunk_upload::received_bytes.eq(chunk_upload::received_bytes + received_bytes))
        .execute(&mut conn)
        .expect("Error updating received bytes for chunk upload");
}

pub fn get_active_chunk_upload(user_id: &str, public_upload_id: &str) -> Option<ActiveChunkUpload> {
    let mut conn = get_connection();

    active_chunk_upload::table
        .filter(active_chunk_upload::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .filter(active_chunk_upload::object_name.eq(Uuid::parse_str(public_upload_id).unwrap()))
        .first::<ActiveChunkUpload>(&mut conn)
        .optional()
        .expect("Error loading active chunk upload")
}

pub fn complete_chunk_upload(user_id: &str, public_upload_id: &str) {
    let mut conn = get_connection();

    diesel::update(chunk_upload::table)
        .filter(chunk_upload::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .filter(chunk_upload::object_name.eq(Uuid::parse_str(public_upload_id).unwrap()))
        .set(chunk_upload::completed_at.eq(chrono::Utc::now()))
        .execute(&mut conn)
        .expect("Error completing chunk upload");
}

pub fn save_uploaded_chunk_information(user_id: &str, object_name: &str, e_tag: &str, part_number: usize) {
    let chunk_info = NewChunkInformation {
        object_name: Uuid::parse_str(object_name).unwrap(),
        part_number: part_number as i32,
        e_tag: e_tag.to_string(),
        user_id: Uuid::parse_str(user_id).unwrap(),
    };

    let mut conn = get_connection();
    diesel::insert_into(chunk_information::table)
        .values(&chunk_info)
        .execute(&mut conn)
        .expect("Error inserting chunk information");
}

pub fn get_uploaded_parts(user_id: &str, object_name: &str) -> Vec<ChunkInformation> {
    let mut conn = get_connection();

    chunk_information::table
        .filter(chunk_information::object_name.eq(Uuid::parse_str(object_name).unwrap()))
        .filter(chunk_information::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .order_by(chunk_information::part_number.asc())
        .load::<ChunkInformation>(&mut conn)
        .expect("Error loading chunk information")
}

pub fn delete_chunks_for_upload(user_id: &str, object_name: &str) {
    let mut conn = get_connection();

    diesel::delete(chunk_information::table)
        .filter(chunk_information::object_name.eq(Uuid::parse_str(object_name).unwrap()))
        .filter(chunk_information::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .execute(&mut conn)
        .expect("Error deleting chunk information");
}

// for failed uploads only, so we do not consume the quota
pub fn delete_chunk_upload_record(user_id: &str, object_name: &str) {
    let mut conn = get_connection();

    diesel::delete(chunk_upload::table)
        .filter(chunk_upload::object_name.eq(Uuid::parse_str(object_name).unwrap()))
        .filter(chunk_upload::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .execute(&mut conn)
        .expect("Error deleting chunk upload record");
}

pub fn get_user_uploads(user_id: &str) -> Vec<UserUpload> {
    let mut conn = get_connection();

    active_uploads::table
        .filter(active_uploads::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .load::<UserUpload>(&mut conn)
        .expect("Error loading user uploads")
}


fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}