use axum::http::{header, HeaderMap};
use reqwest::{Method, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::all::{User, UserId};
use serenity::builder::CreateCommand;
use serenity::interactions_endpoint::Verifier;

use crate::error::AppError;
use crate::routes::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordTokens<S: AsRef<str>> {
    pub access_token: S,
    pub refresh_token: S,
    pub expires_at: i64,
    pub expires_in: i64,
}

pub async fn store_discord_tokens(
    user_id: &UserId,
    tokens: &DiscordTokens<String>,
) -> Result<(), AppError> {
    tracing::info!("{user_id}, {tokens:#?}");
    Ok(())
}

pub async fn discord_request<S: AsRef<str>, T: Serialize + ?Sized>(
    endpoint: S,
    method: Method,
    body: Option<&T>,
    state: &AppState,
) -> Result<Response, AppError> {
    let url = format!("https://discord.com/api/v10/{}", endpoint.as_ref());

    let mut builder = reqwest::Client::new()
        .request(method.clone(), url.as_str())
        .header(
            header::AUTHORIZATION.as_str(),
            format!("Bot {}", { state.bot_token.as_str() }),
        )
        .header(header::USER_AGENT.as_str(), state.user_agent.as_str());

    if let Some(b) = body {
        builder = builder
            .header(
                header::CONTENT_TYPE.as_str(),
                mime::APPLICATION_JSON.as_ref(),
            )
            .json(b);
    }

    let response = builder.send().await?;

    tracing::debug!("response from {method} {url}: {response:#?}");

    Ok(response.error_for_status()?)
}

pub async fn install_global_commands(
    commands: &[CreateCommand],
    state: &AppState,
) -> Result<Value, AppError> {
    let endpoint = format!("applications/{}/commands", state.client_id);
    let response = discord_request(endpoint, Method::PUT, Some(&commands), state).await?;
    Ok(response.json::<Value>().await?)
}

pub async fn create_message<S: AsRef<str>>(
    payload: Value,
    channel_id: S,
    state: &AppState,
) -> Result<Value, AppError> {
    let endpoint = format!("channels/{}/messages", channel_id.as_ref());
    let response = discord_request(endpoint, Method::POST, Some(&payload), state).await?;
    Ok(response.json::<Value>().await?)
}

pub async fn verify_request<S: AsRef<str>>(
    headers: &HeaderMap,
    body: S,
    state: &AppState,
) -> Result<(), AppError> {
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

    Verifier::new(state.public_key.as_str())
        .verify(signature, timestamp, body.as_ref().as_bytes())
        .map_err(|_| AppError::Other("Signature verification failed".to_string()))
}

pub async fn get_oauth_url(state: &AppState) -> Result<(String, String), AppError> {
    let user_state = uuid::Uuid::new_v4().to_string();
    let url = url::Url::parse_with_params(
        "https://discord.com/api/oauth2/authorize",
        &[
            ("client_id", state.client_id.as_str()),
            (
                "redirect_url",
                &format!("{}/discord-oauth-callback", state.base_url),
            ),
            ("response_type", "code"),
            ("state", user_state.as_str()),
            ("scope", "role_connections.write identify"),
            ("prompt", "consent"),
        ],
    )?;

    Ok((url.to_string(), user_state))
}

pub async fn get_oauth_tokens<S: AsRef<str>>(
    code: S,
    state: &AppState,
) -> Result<DiscordTokens<String>, AppError> {
    let response = reqwest::Client::new()
        .post("https://discord.com/api/v10/oauth2/token")
        .form(&[
            ("client_id", state.client_id.as_str()),
            ("client_secret", state.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_ref()),
            (
                "redirect_uri",
                &format!("{}/discord-oauth-callback", state.base_url),
            ),
        ])
        .send()
        .await?
        .error_for_status()?;
    let data = response.json::<DiscordTokens<String>>().await?;
    Ok(data)
}

pub async fn get_user_data(tokens: &DiscordTokens<String>) -> Result<User, AppError> {
    Ok(reqwest::Client::new()
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(tokens.access_token.as_str())
        .send()
        .await?
        .error_for_status()?
        .json::<User>()
        .await?)
}

pub async fn delete_message<S: AsRef<str>>(
    message_id: S,
    channel_id: S,
    state: &AppState,
) -> Result<(), AppError> {
    let endpoint = format!(
        "channels/{}/messages/{}",
        channel_id.as_ref(),
        message_id.as_ref()
    );
    discord_request(endpoint, Method::DELETE, None::<&str>, state).await?;
    Ok(())
}
