


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


diesel::table! {
    chunk_upload (object_name) {
        object_name -> Uuid,
        aws_upload_id -> Text,
        user_id -> Uuid,
        chunk_size -> BigInt,
        received_bytes -> BigInt,
        file_name -> Text,
        file_integrity_algorithm -> Text,
        file_integrity_hash -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
    }
}

// view of chunk_upload where completed_at is null
diesel::table! {
    active_chunk_upload (object_name) {
        object_name -> Uuid,
        aws_upload_id -> Text,
        user_id -> Uuid,
        chunk_size -> BigInt,
        received_bytes -> BigInt,
        file_name -> Text,
        file_integrity_algorithm -> Text,
        file_integrity_hash -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}


diesel::table! {
    chunk_information (object_name, part_number) {
        object_name -> Uuid,
        part_number -> Integer,
        e_tag -> Text,
        user_id -> Uuid,
        created_at -> Timestamptz,
    }
}