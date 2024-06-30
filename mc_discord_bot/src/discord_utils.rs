use std::env::var;
use axum::http::HeaderMap;
use reqwest::{Method, Response};
use serde::Serialize;
use serenity::builder::CreateCommandOption;
use serenity::interactions_endpoint::Verifier;
use lib::AppError;

pub async fn discord_request<S: AsRef<str>, T: Serialize + ?Sized>(
    endpoint: S,
    method: Method,
    body: Option<&T>,
) -> Result<Response, AppError> {
    let bot_token = var("DISCORD_BOT_TOKEN").unwrap_or_default();
    let repo = env!("CARGO_PKG_REPOSITORY");
    let version = env!("CARGO_PKG_VERSION");

    #[cfg(not(debug_assertions))]
    let url = format!("https://discord.com/api/v10/{}", endpoint.as_ref());
    #[cfg(debug_assertions)]
    let url = format!("http://localhost:8081/{}", endpoint.as_ref());

    let mut builder = reqwest::Client::new()
        .request(method, url)
        .header("Authorization", format!("Bot {bot_token}"))
        .header("User-Agent", format!("DiscordBot ({repo}, {version})"));

    if let Some(b) = body {
        builder = builder
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(b);
    }

    Ok(builder.send().await?.error_for_status()?)
}

pub async fn install_global_commands(commands: &[CreateCommandOption]) -> Result<Response, AppError> {
    let app_id = var("DISCORD_CLIENT_ID").unwrap_or_default();
    let endpoint = format!("applications/{app_id}/commands");
    discord_request(endpoint, Method::PUT, Some(&commands)).await
}

pub async fn verify_discord_request(headers: &HeaderMap, body: &str) -> Result<(), AppError> {
    let public_key = var("DISCORD_PUBLIC_KEY")?;
    let signature = headers
        .get("X-Signature-Ed25519")
        .ok_or_else(|| AppError::Other("Missing Discord signature".to_string()))?
        .to_str()
        .map_err(|_| AppError::Other("Invalid Discord signature".to_string()))?;
    let timestamp = headers
        .get("X-Signature-Timestamp")
        .ok_or_else(|| AppError::Other("Missing Discord timestamp".to_string()))?
        .to_str()
        .map_err(|_| AppError::Other("Invalid Discord timestamp".to_string()))?;

    Verifier::new(public_key.as_str())
        .verify(signature, timestamp, body.as_bytes())
        .map_err(|_| AppError::Other("Signature verification failed".to_string()))
}
