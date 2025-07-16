


diesel::table! {
    user_quota (user_id) {
        user_id -> Uuid,
        upload_quota -> BigInt,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}


diesel::table! {
    user_uploads (user_id, resource_id) {
        user_id -> Uuid,
        resource_id -> Uuid,
        file_size -> BigInt,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}


diesel::table! {
    active_uploads (user_id) {
        user_id -> Uuid,
        resource_id -> Uuid,
        file_size -> BigInt,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }    
}
