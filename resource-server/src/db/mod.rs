mod schema;

use diesel::prelude::*;
use uuid::Uuid;

use schema::*;


// provided by the active resources view, which filters out deleted resources
#[derive(Queryable, Selectable)]
#[diesel(table_name = active_resources)]
pub struct Resource {
    pub id: Uuid,
    pub user_id: Uuid,
    pub is_public: bool,
    pub resource_name: String,
    pub resource_type: String,
    pub resource_status: String,
    pub origin_file_path: Option<String>,
    pub base_directory: Option<String>,
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
    pub origin_file_path: String,
}

#[derive(Insertable)]
#[diesel(table_name = app_resource)]
pub struct ResourceStatusUpdate {
    id: Uuid,
    resource_status: String,
}

#[derive(Insertable)]
#[diesel(table_name = app_resource)]
pub struct ResourceTypeUpdate {
    id: Uuid,
    resource_type: String,
}

#[derive(Insertable)]
#[diesel(table_name = app_resource)]
pub struct ResourceStoragePathUpdate {
    id: Uuid,
    base_directory: String,
}

pub fn create_resource(
    resource_uuid: String,
    user_id: String,
    resource_name: String,
    origin_file_path: String,
) {

    let mut conn = get_connection();

    let new_resource = NewResource {
        id: Uuid::parse_str(&resource_uuid).unwrap(),
        user_id: Uuid::parse_str(&user_id).unwrap(),
        is_public: false, 
        resource_name,
        resource_type: "unknown".to_string(), // resource type, can be set later
        resource_status: "pending".to_string(), // resource status, can be set later
        origin_file_path,
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

pub fn update_resource_storage(
    resource_uuid: String,
    base_directory: String,
) {
    let mut conn = get_connection();

    let resource_id = Uuid::parse_str(&resource_uuid).unwrap();

    diesel::update(app_resource::table.filter(app_resource::id.eq(resource_id)))
        .set(app_resource::base_directory.eq(base_directory))
        .execute(&mut conn)
        .expect("Error updating resource storage path");
}

pub fn get_active_resources_by_user_id(user_id: &str) -> Vec<Resource> {
    let mut conn = get_connection();

    active_resources::table
        .filter(active_resources::user_id.eq(Uuid::parse_str(user_id).unwrap()))
        .load::<Resource>(&mut conn)
        .expect("Error loading active resources")
}


fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}