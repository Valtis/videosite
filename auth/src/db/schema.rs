

diesel::table! {
    active_users (id) {
        id -> Uuid,
        email -> Text,
        display_name -> Text,
        password_hash -> Text,

        password_valid_until -> Nullable<Timestamptz>,
    }    
}