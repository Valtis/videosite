mod schema;

use diesel::prelude::*;
use uuid::Uuid;

use schema::*;

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = active_uploads)]
pub struct UserUpload {
    pub user_id: Uuid,
    pub resource_id: Uuid,
    pub file_size: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = user_uploads)]
pub struct NewUserUpload {
    pub user_id: Uuid,
    pub resource_id: Uuid,
    pub file_size: i64,
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