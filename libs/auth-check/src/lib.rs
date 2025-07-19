use std::env;

use std::collections::HashMap;

use axum_extra::extract::CookieJar;
use reqwest;

use base64;

use axum::{
    extract::{
        Request
    }, 
    http::StatusCode, 
    middleware::Next, 
    response::Response
};


#[derive(Debug, Clone, serde::Deserialize)]
pub struct UserInfo {
    #[serde(rename = "sub")]
    pub user_id: String,
    pub display_name: String,
    pub email: String,
}

// implement a tower middleware that fetches the auth token from the cookies or Authorization header, and then delegates
// the authentication to the authorization service

pub async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {

    let client_ip = get_client_ip(&req);

    let token = get_token(&req);

    if let Some(token) = token {
        let res =  is_authenticated(&token, client_ip).await;
        match res {
            Ok(true) => {
                let claims = token.split('.').nth(1).unwrap();
                let decoded = base_64_decode(claims).unwrap();
                let user_info: UserInfo = serde_json::from_str(&decoded).unwrap();

                req.extensions_mut().insert(user_info);
                return Ok(next.run(req).await);
            }
            Ok(false) => {
                return Err(StatusCode::UNAUTHORIZED);
            }
            Err(_) => {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            },
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}


/// Adds UserInfo to the request extensions if the user is authenticated,
///
/// This function will not enforce authentication, it is used merely to enable further checks in the request handlers.
/// (E.g. if user has access to a resource)
/// 
/// # Arguments
/// * `req` - The request to which the UserInfo will be added if the user is authenticated.
///
pub async fn add_user_info_to_request(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {

    let client_ip = get_client_ip(&req);
    let token = get_token(&req);

    if let Some(token) = token {
    let res =  is_authenticated(&token, client_ip).await;
        match res {
            Ok(true) => {
                let claims = token.split('.').nth(1).unwrap();
                // append the padding, as the base64 decoder requires it
                let decoded = base_64_decode(claims).unwrap();
                let user_info = serde_json::from_str(&decoded).unwrap();

                req.extensions_mut().insert::<Option<UserInfo>>(Some(user_info));
            }
            Ok(false) => {
                req.extensions_mut().insert::<Option<UserInfo>>(None);
            }
            Err(_) => {
                req.extensions_mut().insert::<Option<UserInfo>>(None);
            },
        }
    } else {
        req.extensions_mut().insert::<Option<UserInfo>>(None);
    }

    return Ok(next.run(req).await);
}


fn get_token(req: &Request) -> Option<String> {
    // check if we have an authorization header with a valid token
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_value) = auth_header.to_str() {
            if auth_value.starts_with("Bearer ") {
                return Some(auth_value.trim_start_matches("Bearer ").to_string());
            }    
        } 
    }

  
    let cookie_jar = CookieJar::from_headers(req.headers());
    // check if we have a session cookie
    if let Some(cookie) = cookie_jar.get("session") {
        return Some(cookie.value().to_string());
    }
    None
}

async fn is_authenticated(token: &str, client_ip: String) -> Result<bool, reqwest::Error> {
    
    let client = reqwest::Client::new();
    let auth_server_url = env::var("AUTH_SERVICE_URL").expect("AUTH_SERVICE_URL must be set");

    let mut map = HashMap::new();
    map.insert("token", token);

    let response = client
        .post(&format!("{}/auth/verify", auth_server_url))
        .header("Content-Type", "application/json")
        .header("X-Client-IP", client_ip)
        .json(&map)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(true) 
    } else {
        Ok(false)
    }
}

fn get_client_ip(req: &Request) -> String {
    // Prioritize Amazon headers for client IP
    // This uses the X-Forwarded-For header, which is set by the load balancer.
    // Grab the leftmost IP address from the comma-separated list
    if let Some(forwarded_for) = req.headers().get("X-Forwarded-For") {
        if let Ok(ip) = forwarded_for.to_str() {
            return ip.split(',').next().unwrap_or("unknown").trim().to_string();
        }
    }   

    // If not set, we are running in thhe local dev env which uses Nginx. This is configured
    // to set the X-Real-IP header to the client IP
    if let Some(forwarded_for) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = forwarded_for.to_str() {
            return ip.to_string();
        }
    }

    // remote address is not useful, we will always get the IP of the proxy server,
    // so we give up and return "unknown"
    "unknown".to_string()
}



fn base_64_decode(input: &str) -> Result<String, base64::DecodeError> {

    let config = base64::engine::general_purpose::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent);

    let alphabet = base64::alphabet::STANDARD;
    let engine = base64::engine::GeneralPurpose::new(&alphabet, config);

    base64::Engine::decode(
        &engine,
        input.as_bytes()
    ).map(|bytes| String::from_utf8(bytes).unwrap())
}