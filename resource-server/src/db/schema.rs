
// resource table
diesel::table! {
    app_resource (id) {
        id -> Uuid,
        user_id -> Uuid,
        is_public -> Bool,
        resource_name -> Varchar,
        resource_type -> Varchar,
        resource_status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

// video metadata table
diesel::table! {
    video_metadata (resource_id) {
        resource_id -> Uuid,
        width -> Integer,
        height -> Integer,
        duration_seconds -> Integer,
        bit_rate -> Integer,
        frame_rate -> Float,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }    
}

diesel::table! {
    transfer_quota (id) {
        id -> Int4,
        quota_used -> BigInt,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}