mod schema;

use diesel::prelude::*;
use uuid::Uuid;

use schema::active_users;




#[derive(Queryable, Selectable)]
#[diesel(table_name = active_users)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
    pub password_valid_until: Option<chrono::DateTime<chrono::Utc>>,
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

pub fn get_user_by_id(user_id: &str) -> Option<User> {
    use schema::active_users::dsl::*;

    let mut connection = get_connection();
    let user_id_uuid = Uuid::parse_str(user_id).unwrap();
    let result = active_users
        .filter(id.eq(user_id_uuid))
        .first::<User>(&mut connection)
        .optional()
        .expect("Error loading user");

    result
}

pub fn update_user_password(user_id: Uuid, new_password_hash: &str) -> Result<(), diesel::result::Error> {
    use schema::active_users::dsl::*;

    let mut connection = get_connection();

    diesel::update(active_users.filter(id.eq(user_id)))
        .set((
            password_hash.eq(new_password_hash),
            password_valid_until.eq::<Option<chrono::DateTime<chrono::Utc>>>(None))
        )
        .execute(&mut connection)?;

    Ok(())
}

fn get_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}