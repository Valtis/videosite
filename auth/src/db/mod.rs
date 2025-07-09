mod schema;

use diesel::prelude::*;
use uuid::Uuid;

use schema::active_users;




#[derive(Queryable, Selectable)]
#[diesel(table_name = active_users)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
}

pub fn get_user_by_email(user_email: &str) -> Option<User> {

    use schema::active_users::dsl::*;

    let mut connection = get_connection();

    let result = active_users
        .filter(email.eq(user_email))
        .first::<User>(&mut connection)
        .optional()
        .expect("Error loading user");

    result
}

fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}