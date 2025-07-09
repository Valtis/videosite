

diesel::table! {
    active_users (id) {
        id -> Uuid,
        email -> Text,
        display_name -> Text,
        password_hash -> Text,
    }    
}