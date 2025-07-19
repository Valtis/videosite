
// resource table
diesel::table! {
    audit_event (id) {
        id -> Integer,
        user_id -> Uuid,
        client_ip -> Varchar,
        event_action -> Varchar,
        action_target -> Nullable<Uuid>,
        additional_info -> Nullable<Jsonb>,
        event_timestamp -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

