mod schema;

use diesel::prelude::*;
use uuid::Uuid;

use schema::*;


// provided by the active resources view, which filters out deleted resources
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = active_resources)]
#[allow(dead_code)]
pub struct Resource {
    pub id: Uuid,
    pub user_id: Uuid,
    pub is_public: bool,
    pub resource_name: String,
    pub resource_type: String,
    pub resource_status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// much like the Resource struct, but includes deleted_at field
#[derive(Insertable)]
#[diesel(table_name = app_resource)]
#[allow(dead_code)]
pub struct NewResource {
    pub id: Uuid,
    pub user_id: Uuid,
    pub is_public: bool,
    pub resource_name: String,
    pub resource_type: String,
    pub resource_status: String,
}

#[derive(Insertable)]
#[diesel(table_name = transfer_quota)]
#[allow(dead_code)]
pub struct NewTransferQuota {
    pub quota_used: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub fn create_resource(
    resource_uuid: String,
    user_id: String,
    resource_name: String
) {

    let mut conn = get_connection();

    let new_resource = NewResource {
        id: Uuid::parse_str(&resource_uuid).unwrap(),
        user_id: Uuid::parse_str(&user_id).unwrap(),
        is_public: false, 
        resource_name,
        resource_type: "unknown".to_string(), // resource type, can be set later
        resource_status: "pending".to_string(), // resource status, can be set later
    };

    diesel::insert_into(app_resource::table)
        .values(&new_resource)
        .execute(&mut conn)
        .expect("Error creating new resource");
}

pub fn update_resource_status(
    resource_uuid: String,
    resource_status: String,
) {
    let mut conn = get_connection();

    let resource_id = Uuid::parse_str(&resource_uuid).unwrap();

    diesel::update(app_resource::table.filter(app_resource::id.eq(resource_id)))
        .set(app_resource::resource_status.eq(resource_status))
        .execute(&mut conn)
        .expect("Error updating resource status");
}

pub fn update_resource_type(
    resource_uuid: String,
    resource_type: String,
) {
    let mut conn = get_connection();

    let resource_id = Uuid::parse_str(&resource_uuid).unwrap();

    diesel::update(app_resource::table.filter(app_resource::id.eq(resource_id)))
        .set(app_resource::resource_type.eq(resource_type))
        .execute(&mut conn)
        .expect("Error updating resource type");
}

pub fn update_resource_public_status(
    resource_uuid: &str,
    is_public: bool,
) {
    let mut conn = get_connection();

    let resource_id = Uuid::parse_str(&resource_uuid).unwrap();

    diesel::update(app_resource::table.filter(app_resource::id.eq(resource_id)))
        .set(app_resource::is_public.eq(is_public))
        .execute(&mut conn)
        .expect("Error updating resource public status");
}

pub fn get_active_resources_by_user_id(user_id: &str) -> Vec<Resource> {
    let mut conn = get_connection();

    active_resources::table
        .filter(active_resources::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .order(active_resources::created_at.desc())
        .load::<Resource>(&mut conn)
        .expect("Error loading active resources")
}

pub fn get_active_resource_by_id(resource_id: &str) -> Option<Resource> {
    let mut conn = get_connection();

    active_resources::table
        .filter(active_resources::id.eq(Uuid::parse_str(resource_id).unwrap()))
        .first::<Resource>(&mut conn)
        .ok()
}



pub fn get_used_daily_quota() -> Option<i64> {
    let mut conn = get_connection();

    // get the quota used for today. Sort by created_at to get the latest entry and 
    // check that it is from today
    transfer_quota::table
        .select(transfer_quota::quota_used)
        .filter(transfer_quota::created_at.ge(chrono::Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap()))
        .order(transfer_quota::created_at.desc())
        .first(&mut conn)
        .optional()
        .expect("Error checking existing daily quota")
}

/// Update the daily quota used by a specified amount
/// If the daily quota does not exist, it will be created.
/// 
/// # Arguments
/// /// * `amount_used` - The amount to add to the daily quota used
/// /// # Returns
/// /// Result<(), String> - Ok if the update was successful, Err if there was an error
pub fn update_daily_quota(amount_used: i64) -> Result<(), String> {

    if amount_used <= 0 {
        return Err("Amount used must be greater than zero".to_string());
    }

    let mut conn = get_connection();

    let today = chrono::Utc::now().date_naive();
    // Check if a daily quota entry exists for today
    let existing_quota = get_used_daily_quota();

    match existing_quota {
        Some(quota_used) => {
            // Update existing entry
            diesel::update(
                transfer_quota::table
                .filter(transfer_quota::created_at.ge(today.and_hms_opt(0, 0, 0).unwrap())))
                .set(transfer_quota::quota_used.eq(quota_used + amount_used))
                .execute(&mut conn)
                .map_err(|err| {
                    format!("Error updating daily quota: {}", err)
                })?;
        }
        None => {
            // Insert new entry
            let new_quota = NewTransferQuota {
                quota_used: amount_used,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            diesel::insert_into(transfer_quota::table)
                .values(&new_quota)
                .execute(&mut conn)
                .map_err(|err| {
                    format!("Error inserting new daily quota: {}", err)
                })?;
        }
    }

    Ok(())
}  


fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}