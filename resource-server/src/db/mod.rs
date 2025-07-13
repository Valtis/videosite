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
    pub origin_file_path: String,
    pub base_directory: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

// much like the Resource struct, but includes deleted_at field
#[derive(Queryable, Selectable, Insertable)]
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


fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}