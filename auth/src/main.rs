mod db;

use std::env;
use std::time::Duration;
use std::time::{ SystemTime };

use axum::{
    routing::{get, post},
    Router,
    Json,
    http::{
        HeaderMap,
        HeaderValue,
    },
    http::header::{
        SET_COOKIE,
    },
    http::StatusCode,
    response::{IntoResponse},
};

use axum_extra::extract::cookie::CookieJar;

use josekit::JoseError;
use josekit::{jws::{JwsHeader, HS256}, jwt::{self, JwtPayload}, Value};

use scrypt::{
    password_hash::{PasswordHash, PasswordVerifier},
    Scrypt,
};

use tracing;
use tracing_subscriber::filter;
use tracing_subscriber;

use serde::{Deserialize, Serialize, ser::SerializeStruct };

use db::{User, get_user_by_email};




#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct UserInfo {
    display_name: String,
    email: String,
}

#[derive(Deserialize)]
struct TokenVerificationRequest {
    token: String,
}


struct LoginResponse {
    res: Result<String, String>,
}

impl Serialize for LoginResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("LoginResponse", 1)?;
        match &self.res {
            Ok(msg) => {
                state.serialize_field("msg", msg)?;
            }
            Err(err) => {
                state.serialize_field("err", err)?;
            }
        }
        state.end()

    }
}


/// Verifies a JWT token provided via cookie.
/// 
///  This is used by the frontend to check if the user is logged in.
/// 
/// # Arguments
/// * `cookie_jar`: The cookie jar containing the cookies sent by the client.
////// # Returns
/// * `StatusCode::OK` if the token is valid.
/// * `StatusCode::UNAUTHORIZED` if the token is missing or invalid.
async fn verify_jwt_via_cookie(cookie_jar: CookieJar) -> StatusCode {
    let token = match cookie_jar.get("session") {
        Some(cookie) => cookie.value().to_string(),
        None => { 
            tracing::debug!("No session cookie found");
            return StatusCode::UNAUTHORIZED;
        }
    };

    if verify_token(&token) {
        tracing::debug!("Token verification successful");
        StatusCode::OK
    } else {
        tracing::debug!("Token verification failed");
        StatusCode::UNAUTHORIZED
    }
}

/// Verifies a JWT token provided in the request body.
/// 
/// This is used by the sibling services to verify the user is who they claim to be.
/// 
/// # Arguments
/// * `payload`: The request body containing the token to verify.
/// # Returns
/// * `StatusCode::OK` if the token is valid.
/// * `StatusCode::UNAUTHORIZED` if the token is invalid.car
///  
async fn verify_jwt(payload: Json<TokenVerificationRequest>) -> StatusCode {
    if verify_token(&payload.token) {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    }
}

/// Handles user login by validating credentials and issuing a JWT token.
/// 
/// The JWT is provided in a cookie to the client. This is to prevent Javascript from accessing the token directly.
/// 
/// # Arguments
/// * `payload`: The request body containing the username and password.
/// # Returns
/// * `StatusCode::OK` with a JSON response containing the result of the login attempt
/// * `StatusCode::UNAUTHORIZED` if the credentials are invalid.
/// 
async fn login_handler(Json(payload): Json<LoginRequest>) -> impl IntoResponse {

    // TODO: Fetch user from database and validate credentials
    // for now, hardcoded test user

    let user_opt = get_user_by_email(&payload.username);

    let mut headers = HeaderMap::new();
    
    let cookie;
    if let Some(user) = user_opt {
        if password_equals(&user.password_hash, &payload.password) {
            tracing::debug!("User {} logged in successfully", user.id);
            cookie = format!("session={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=43200", generate_jwt(user));
        } else {
            tracing::debug!("Invalid password for user: {}", user.id);
            let json = Json(LoginResponse {
                res: Err("Invalid username or password".to_string()),
            });
            return (StatusCode::OK, headers, json).into_response();
        } 
       

    } else {
        tracing::debug!("User not found: {}", payload.username);
        let json = Json(LoginResponse {
            res: Err("Invalid username or password".to_string()),
        });
        return (StatusCode::OK, headers, json).into_response();
    }


    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).unwrap(),
    );

    let json = Json(LoginResponse {
        res: Ok("Success".to_string()),
    });

    (StatusCode::OK, headers, json).into_response()
}

/// Generates a JWT token for the given username.
/// 
/// # Arguments
/// * `username`: The username for which to generate the token.
/// # Returns
/// * A JWT token as a `String`.
/// # Panics
/// * If the environment variables `SIGNING_KEY`, `ISSUER`, or `AUDIENCE` are not set.
/// * If the JWT encoding fails.
///
fn generate_jwt(user: User) -> String {
    let secret_key = env::var("SIGNING_KEY").expect("SIGNING_KEY environment variable not set");
    let issuer = env::var("ISSUER").expect("ISSUER environment variable not set");
    let audience = env::var("AUDIENCE").expect("AUDIENCE environment variable not set");

    let now = SystemTime::now();

    let mut header = JwsHeader::new();
    header.set_token_type("JWT");

    let mut payload = JwtPayload::new();

    payload.set_issuer(issuer.as_str());
    payload.set_audience(vec![audience.as_str()]);
    payload.set_subject(user.id);
    payload.set_issued_at(&now);
    payload.set_not_before(&now);
    payload.set_claim("email", Some(Value::String(user.email))).expect("Failed to set email claim");
    payload.set_claim("display_name", Some(Value::String(user.display_name))).expect("Failed to set display_name claim");


    // FIXME: Move the expiration time to an environment variable
    payload.set_expires_at(&now.checked_add(Duration::from_secs(12*60*60)).unwrap()); 

    let signer = HS256.signer_from_bytes(secret_key.as_bytes())
        .expect("Failed to create signer from secret key");

    jwt::encode_with_signer(&payload, &header, &signer)
        .expect("Failed to encode JWT")
}


/// Verifies a JWT token.
/// # Arguments
/// * `token`: The JWT token to verify.
/// # Returns
/// * `true` if the token is valid.
/// * `false` if the token is invalid.
/// # Panics
/// * If the environment variables `SIGNING_KEY`, `ISSUER`, or `AUDIENCE` are not set.
/// * If the JWT decoding fails.
fn verify_token(token: &str) -> bool {
    let issuer = env::var("ISSUER").expect("ISSUER environment variable not set");
    let audience = env::var("AUDIENCE").expect("AUDIENCE environment variable");
    let now = SystemTime::now();


    if let Ok((payload, _)) = get_payload(token) {

        if payload.expires_at().is_none() || payload.expires_at().unwrap() <= now {
            tracing::debug!("Token verification failed: Token has expired");
            return false;
        }

        if payload.issuer().is_none() || payload.issuer().unwrap() != &issuer {
            tracing::debug!("Token verification failed: Invalid issuer");
            return false;
        }

        if payload.audience().is_none() || !payload.audience().unwrap().contains(&audience.as_str()) {
            tracing::debug!("Token verification failed: Invalid audience");
            return false;
        }

        if payload.subject().is_none() {
            tracing::debug!("Token verification failed: Missing subject");
            return false;
        }

        if payload.issued_at().is_none() || payload.issued_at().unwrap() > now {
            tracing::debug!("Token verification failed: Invalid issued at time");
            return false;
        }

        if payload.not_before().is_none() || payload.not_before().unwrap() > now {
            tracing::debug!("Token verification failed: Invalid not before time");
            return false;
        }

        return true;
    } else {
        tracing::debug!("Token verification failed: Invalid token");
        false
    }
}

fn get_payload(token: &str) -> Result<(JwtPayload, JwsHeader), JoseError> {
    let secret_key = env::var("SIGNING_KEY").expect("SIGNING_KEY environment variable not set");
    let verifier = HS256.verifier_from_bytes(secret_key.as_bytes())
        .expect("Failed to create verifier from secret key");

    jwt::decode_with_verifier(&token, &verifier)
}

async fn user_info(cookie_jar: CookieJar) -> impl IntoResponse {
    let token = match cookie_jar.get("session") {
        Some(cookie) => cookie.value().to_string(),
        None => { 
            tracing::debug!("No session cookie found");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    if verify_token(&token) {
        let payload = get_payload(&token).expect("Failed to get payload from token").0;

        return Json(
            UserInfo { 
                display_name: payload.claim("display_name").unwrap().as_str().unwrap().to_string(),
                email: payload.claim("email").unwrap().as_str().unwrap().to_string(),

            }).into_response();
   }

    StatusCode::UNAUTHORIZED.into_response()
}



/// Checks if the provided password matches the hashed password.
/// 
/// # Arguments
/// * `hash`: The hashed password.
/// * `password`: The password to check.
/// 
/// # Returns
/// * `true` if the password matches the hash.
/// * `false` if the password does not match the hash.
/// 
/// # Panics
/// * If the password hash cannot be parsed.
fn password_equals(hash: &str, password: &str) -> bool {
    let parsed_hash = PasswordHash::new(hash).expect("Failed to parse password hash");
    Scrypt.verify_password(password.as_bytes(), &parsed_hash).is_ok()
}




#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        .pretty()
        .with_max_level(filter::LevelFilter::DEBUG)
        .init();

    let app = Router::new()
        .route("/auth/health", get(|| async { "OK" }))
        .route("/auth/status", get(verify_jwt_via_cookie))
        .route("/auth/verify", post(verify_jwt))
        .route("/auth/login", post(login_handler))
        .route("/auth/info", get(user_info))
        ;
        

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}