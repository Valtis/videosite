use axum::{
    routing::get,
    Router,
};

use axum_extra::extract::cookie::CookieJar;
use reqwest;
use urlencoding::encode as url_encode;

use std::env;

use tracing;
use tracing_subscriber::filter;

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct MetadataResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(flatten)]
    resource_metadata: ResourceMetadata,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[allow(dead_code)]
enum ResourceMetadata {
    Video{ width: i32, height: i32, duration_seconds: i32, bit_rate: i32, frame_rate: f32 },
}

// generate a stub page with open graph and oembed tags for a given resource id
// fetch metadata from resource server
#[axum::debug_handler]
async fn embed_html(
    cookies: CookieJar,
    axum::extract::Path(resource_id): axum::extract::Path<String>,
) -> Result<axum::response::Html<String>, axum::http::StatusCode> {
    generate_html(&resource_id, cookies).await
}

#[axum::debug_handler]
async fn embed_html_query(
    cookies: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Html<String>, axum::http::StatusCode> {
    if let Some(resource_id) = params.get("resource_id") {
        generate_html(resource_id, cookies).await
    } else {
        Err(axum::http::StatusCode::BAD_REQUEST)
    }
}




#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_level(true)
        .pretty()
        .with_max_level(filter::LevelFilter::INFO)
        .init();

    tracing::info!("Starting embed tag service...");


    let app = Router::new()
        .route("/embed-service/health", get(|| async { "OK" }))
        .route("/embed-service/{resource_id}/embed.html", get(embed_html))
        // laziness - do not want to rewrite the url in AWS
        .route("/player.html", get(embed_html_query));



    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await
        .expect("Failed to bind TCP listener");


    tracing::info!("Listening on port {}", port);
    tokio::join!(
        axum::serve(
            listener, 
            app)
    ).0.unwrap();

}


async fn generate_html(resource_id: &str, cookies: CookieJar) -> Result<axum::response::Html<String>, axum::http::StatusCode> {
    let metadata_url = format!(
        "{}/resource/{}/metadata",
        env::var("RESOURCE_SERVER_URL").expect("RESOURCE_SERVER_URL must be set"),
        resource_id
    );

    let domain = env::var("DOMAIN_URL").expect("DOMAIN_URL must be set");

    
    // get metadata. Forward any cookies we have, as we may have received a session cookie
    // from the user agent.

    let hyper_cookies = cookies
        .iter()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<String>>()
        .join("; ");

    tracing::info!("DEBUG: Using url: {}, with cookies: {}", metadata_url, hyper_cookies);
    let metadata_response = reqwest::Client::new()
        .get(&metadata_url)
        .header("Cookie", hyper_cookies)
        .send()
        .await
        .map_err(|err| {
            tracing::error!("Error fetching metadata from resource server: {}", err);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Metadata response status: {}", metadata_response.status());
    if !metadata_response.status().is_success() {
        tracing::error!("Error fetching metadata from resource server: {}", metadata_response.status());
        return Err(axum::http::StatusCode::NOT_FOUND);
    }

    let metadata: MetadataResponse = serde_json::from_str(&metadata_response.text().await.map_err(|err| {
        tracing::error!("Error reading metadata response text: {}", err);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?).map_err(|err| {
        tracing::error!("Error parsing metadata response JSON: {}", err);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // placeholder: We have no descriptions available yet, use lorem ipsum
    let description = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.";

    tracing::info!("Fetched metadata: {:?}", metadata);

    let html = match metadata.resource_metadata {
        ResourceMetadata::Video{ width, height, .. } => {
            let player_url = format!("{}/player.html?resource_id={}", domain, metadata.id);
            let url_encoded_player_url = url_encode(&player_url);


            format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <meta property="og:title" content="{title}" /> 
    <meta property="og:type" content="video.other" />
    <meta property="og:image" content="{thumbnail_url}" />
    <meta property="og:url" content="{url}" />
    <meta property="og:description" content="{description}" />
    <meta property="og:video" content="{video_url}" />
    <meta property="og:video:type" content="text/html" />
    <meta property="og:video:width" content="{width}" />
    <meta property="og:video:height" content="{height}" />
    <link rel="alternate" type="application/json+oembed" href="{oembed_url}" title="{title}" />
</head>
<body>
</body>
</html>
"#, 
    title = metadata.name,
    thumbnail_url = format!("{}/resource/{}/thumbnail.jpg", domain, metadata.id),
    url = player_url,
    description = description,
    video_url = format!("{}/resource/{}/master.m3u8", domain, metadata.id),
    width = width,
    height = height,
    oembed_url = format!("{}/resource/oembed.json?url={}", domain, url_encoded_player_url)
            )
        }
        // todo - other resource types
    };

    Ok(axum::response::Html(html.to_string()))
}