
// resource table
diesel::table! {
    app_resource (id) {
        id -> Uuid,
        user_id -> Uuid,
        is_public -> Bool,
        resource_name -> Varchar,
        resource_type -> Varchar,
        resource_status -> Varchar,
        origin_file_path -> Varchar,
        base_directory -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

// resource view, filters out deleted resources
diesel::table! {
    active_resources (id) {
        id -> Uuid,
        user_id -> Uuid,
        is_public -> Bool,
        resource_name -> Varchar,
        resource_type -> Varchar,
        resource_status -> Varchar,
        origin_file_path -> Varchar,
        base_directory -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }    
}

