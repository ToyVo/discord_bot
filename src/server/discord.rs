use crate::error::AppError;
use crate::server::{models::DiscordTokens, AppState};
use anyhow::Context;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};
use reqwest::Method;
use serde::Serialize;
use serde_json::{json, Value};
use serenity::{
    all::{
        CommandDataOptionValue, CommandInteraction, CreateInteractionResponseFollowup, Interaction,
        InteractionResponseFlags, User,
    },
    builder::{
        CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    },
    interactions_endpoint::Verifier,
    json,
};
use std::collections::HashMap;
use tokio::process::Command;

pub async fn interactions(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Request received: {headers:?} {body}");

    // Parse request body and verifies incoming requests
    if verify_request(&headers, &body, &state).await.is_err() {
        return Ok((StatusCode::UNAUTHORIZED, Json(json!({}))));
    }

    let payload = match Json::<Interaction>::from_bytes(body.as_bytes()) {
        Ok(payload) => payload.0,
        Err(e) => {
            tracing::error!("Could not parse body: {e}");
            return Ok((StatusCode::BAD_REQUEST, Json(json!({}))));
        }
    };

    match payload {
        Interaction::Ping(_) => {
            tracing::info!("Received discord ping request, Replying pong");
            Ok((
                StatusCode::OK,
                Json(json::to_value(CreateInteractionResponse::Pong)?),
            ))
        }
        Interaction::Command(command_payload) => {
            match handle_slash_command(command_payload, state).await {
                Ok(value) => Ok((StatusCode::OK, Json(value))),
                Err(e) => {
                    tracing::error!("Slash command error {e}");
                    Ok((StatusCode::OK, Json(json!({}))))
                }
            }
        }
        Interaction::Autocomplete(_) => Ok((StatusCode::NOT_IMPLEMENTED, Json(json!({})))),
        Interaction::Component(_) => Ok((StatusCode::NOT_IMPLEMENTED, Json(json!({})))),
        Interaction::Modal(_) => Ok((StatusCode::NOT_IMPLEMENTED, Json(json!({})))),
        _ => Ok((StatusCode::BAD_REQUEST, Json(json!({})))),
    }
}

pub async fn verify_user(
    jar: SignedCookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let (url, state) = get_oauth_url(&state).await?;
    Ok((
        StatusCode::FOUND,
        jar.add(Cookie::new("clientState", state)),
        Redirect::to(url.as_str()),
    ))
}

pub async fn oauth_callback(
    Query(query): Query<HashMap<String, String>>,
    jar: SignedCookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let code = query.get("code").context("Code not on query")?;
    let discord_state = query.get("state").context("state not on query")?;
    let client_state = jar.get("clientState").context("Cookie not set")?;

    if client_state.value() != discord_state {
        return Ok(StatusCode::UNAUTHORIZED);
    }

    let tokens = &get_oauth_tokens(code, &state).await?;
    let me_data = get_user_data(tokens).await?;
    let user_id = me_data.id;
    let expires_at = chrono::Utc::now() + std::time::Duration::from_millis(tokens.expires_in);
    let stored_tokens = DiscordTokens {
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token.clone(),
        expires_at,
        expires_in: tokens.expires_in,
    };
    // TODO: store in DB
    tracing::info!("{user_id} {stored_tokens:?}");
    Ok(StatusCode::OK)
}

pub async fn handle_slash_command(
    payload: CommandInteraction,
    state: AppState,
) -> Result<Value, AppError> {
    tracing::info!("Received discord slash command request, {:?}", &payload);
    for option in payload.data.options {
        // TODO: single source of truth for this and install_global_commands
        match (
            payload.data.name.as_str(),
            option.name.as_str(),
            option.value,
        ) {
            (
                "minecraft-geyser" | "minecraft-modded" | "terraria",
                "action",
                CommandDataOptionValue::String(s),
            ) if s == "restart" || s == "stop" => {
                let server = payload.data.name.clone();
                let action = s.clone();
                tokio::spawn(async move {
                    let server = payload.data.name;
                    let action = s.as_str();
                    let service_name = format!("arion-{server}.service");
                    let content = match Command::new("systemctl")
                        .args(if let Some(host) = &state.cloud_ssh_host {
                            vec!["--host", host, action, service_name.as_str()]
                        } else {
                            vec![action, service_name.as_str()]
                        })
                        .output()
                        .await
                    {
                        Ok(_) => {
                            format!("Successfully {action}ed {server} server")
                        }
                        Err(e) => {
                            tracing::error!("Could not {action} {server} server: {e}");
                            format!("There was an issue {action}ing {server} server")
                        }
                    };
                    if let Err(e) =
                        replace_initial_interaction_response(content, payload.token, &state).await
                    {
                        tracing::error!("Error submitting followup {e:?}")
                    }
                });
                return Ok(json::to_value(CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "Successfully requested {action} of {server} server"
                        ))
                        .flags(InteractionResponseFlags::SUPPRESS_NOTIFICATIONS),
                ))?);
            }
            (_, _, _) => {}
        }
    }
    Ok(json!({}))
}

pub async fn discord_request<S: AsRef<str>, T: Serialize + ?Sized>(
    endpoint: S,
    method: Method,
    body: Option<&T>,
    state: &AppState,
) -> Result<Option<Value>, AppError> {
    let url = format!("https://discord.com/api/v10/{}", endpoint.as_ref());

    let mut builder = reqwest::Client::new()
        .request(method.clone(), url.as_str())
        .header(
            header::AUTHORIZATION.as_str(),
            format!("Bot {}", { state.discord_token.as_str() }),
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

    tracing::debug!("response from {method} {url}: {response:?}");

    let content_type = response.headers().get(header::CONTENT_TYPE.as_str());
    if content_type.is_some()
        && content_type.unwrap().to_str().unwrap() == mime::APPLICATION_JSON.as_ref()
    {
        let body = response.json::<Value>().await?;
        tracing::debug!("response body from {method} {url}: {body}");
        return Ok(Some(body));
    }

    Ok(None)
}

pub async fn install_global_commands(
    commands: &[CreateCommand],
    state: &AppState,
) -> Result<Value, AppError> {
    let endpoint = format!("applications/{}/commands", state.client_id);
    let response = discord_request(endpoint, Method::PUT, Some(&commands), state).await?;
    Ok(response.context("Response not found from installing commands")?)
}

pub async fn create_message<S: AsRef<str>>(
    payload: CreateMessage,
    channel_id: S,
    state: &AppState,
) -> Result<Value, AppError> {
    let endpoint = format!("channels/{}/messages", channel_id.as_ref());
    let response = discord_request(endpoint, Method::POST, Some(&payload), state).await?;
    let json = response.context("Response not found from creating message")?;
    tracing::info!(
        "Message created {}",
        json.get("id")
            .context("id not found")?
            .as_str()
            .context("failed to parse as str")?
    );
    Ok(json)
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
                &format!("{}/api/discord/oauth-callback", state.base_url),
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

pub async fn replace_initial_interaction_response<S: AsRef<str>>(
    content: impl Into<String>,
    token: S,
    state: &AppState,
) -> Result<(), AppError> {
    discord_request(
        format!(
            "webhooks/{}/{}/messages/@original",
            state.client_id,
            token.as_ref()
        ),
        Method::PATCH,
        Some(&CreateInteractionResponseFollowup::new().content(content)),
        state,
    )
    .await?;
    Ok(())
}
