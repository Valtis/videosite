use std::env;

use std::collections::HashMap;

use axum_extra::extract::CookieJar;
use reqwest;

use axum::{
    extract::{
        Request
    }, 
    http::StatusCode, 
    middleware::Next, 
    response::Response
};

// implement a tower middleware that fetches the auth token from the cookies or Authorization header, and then delegates
// the authentication to the authorization service

pub async fn auth_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    // check if we have an authorization header with a valid token
    let mut token = None;
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_value) = auth_header.to_str() {
            if auth_value.starts_with("Bearer ") {
                token = Some(auth_value.trim_start_matches("Bearer ").to_string());
            }    
        } 
    }

    if token.is_none() {
        let cookie_jar = CookieJar::from_headers(req.headers());
        // check if we have a session cookie
        if let Some(cookie) = cookie_jar.get("session") {
            token = Some(cookie.value().to_string());
        }
    }


    if let Some(token) = token {
        let res =  is_authenticated(&token).await;
        match res {
            Ok(true) => return Ok(next.run(req).await),
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





async fn is_authenticated(token: &str) -> Result<bool, reqwest::Error> {
    
    let client = reqwest::Client::new();
    let auth_server_url = env::var("AUTH_SERVICE_URL").expect("AUTH_SERVICE_URL must be set");

    let mut map = HashMap::new();
    map.insert("token", token);

    let response = client
        .post(&format!("{}/auth/verify", auth_server_url))
        .header("Content-Type", "application/json")
        .json(&map)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(true) 
    } else {
        Ok(false)
    }
}