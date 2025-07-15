mod schema;

use diesel::prelude::*;
use uuid::Uuid;

use schema::*;


// provided by the active resources view, which filters out deleted resources
#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = active_resources)]
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
pub struct NewResource {
    pub id: Uuid,
    pub user_id: Uuid,
    pub is_public: bool,
    pub resource_name: String,
    pub resource_type: String,
    pub resource_status: String,
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
    user_id: &str,
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


fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}