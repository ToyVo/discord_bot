use anyhow::Context;
use axum::extract::{FromRef, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, Key};
use axum_extra::extract::SignedCookieJar;
use serde_json::{json, Value};
use serenity::all::InteractionType;
use serenity::builder::CreateInteractionResponse;
use serenity::json;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::{ServeDir, ServeFile};
use dioxus::prelude::*;

use crate::error::AppError;
use crate::discord_utils;
use crate::discord_utils::DiscordTokens;
use crate::handlers::handle_slash_command;

#[derive(Clone)]
pub struct AppState(pub Arc<InnerState>);

// deref so you can still access the inner fields easily
impl Deref for AppState {
    type Target = InnerState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.0.key.clone()
    }
}

pub struct InnerState {
    pub key: Key,
    pub public_key: String,
    pub client_secret: String,
    pub client_id: String,
    pub bot_token: String,
    pub base_url: String,
    pub user_agent: String,
    pub minecraft_service_name: String,
    pub terraria_service_name: String,
    pub tshock_base_url: String,
    pub minecraft_rcon_address: String,
    pub minecraft_rcon_password: String,
    pub tshock_token: String,
    pub terraria_players: RwLock<Vec<String>>,
    pub discord_terraria_channel_id: String,
    pub discord_minecraft_channel_id: String,
}

pub fn app() -> Router<AppState> {
    Router::new()
        .route("/api/interactions", post(interactions))
        .route("/verify-user", get(verify_user))
        .route("/discord-oauth-callback", get(discord_oauth_callback))
        .route_service(
            "/terms-of-service",
            ServeFile::new("assets/terms-of-service.html"),
        )
        .route_service(
            "/privacy-policy",
            ServeFile::new("assets/privacy-policy.html"),
        )
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/modpack", ServeDir::new("modpack"))
        .fallback_service(
            ServeDir::new("public").not_found_service(get(app_endpoint)),
        )
}

async fn app_endpoint() -> Html<String> {
    // render the rsx! macro to HTML
    Html(dioxus_ssr::render_element(rsx! { div { "hello world!" } }))
}

pub async fn interactions(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Request received: {headers:#?} {body}");

    // Parse request body and verifies incoming requests
    if discord_utils::verify_request(&headers, &body, &state)
        .await
        .is_err()
    {
        return Ok((StatusCode::UNAUTHORIZED, Json(json!({}))));
    }

    let payload = match Json::<Value>::from_bytes(body.as_bytes()) {
        Ok(payload) => payload,
        Err(e) => {
            tracing::error!("Could not parse body\n{e:#?}");
            return Ok((StatusCode::BAD_REQUEST, Json(json!({}))));
        }
    };

    let request_type = payload
        .get("type")
        .and_then(|s| s.as_u64())
        .map(|n| InteractionType::from(n as u8));

    match request_type {
        Some(InteractionType::Ping) => {
            tracing::info!("Received discord ping request, Replying pong");
            Ok((
                StatusCode::OK,
                Json(json::to_value(CreateInteractionResponse::Pong)?),
            ))
        }
        Some(InteractionType::Command) => match handle_slash_command(payload, state).await {
            Ok(value) => Ok((StatusCode::OK, Json(value))),
            Err(e) => {
                tracing::error!("Slash command error {e}");
                Ok((StatusCode::OK, Json(json!({}))))
            }
        },
        _ => Ok((StatusCode::OK, Json(json!({})))),
    }
}

pub async fn verify_user(
    jar: SignedCookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let (url, state) = discord_utils::get_oauth_url(&state).await?;
    Ok((
        StatusCode::FOUND,
        jar.add(Cookie::new("clientState", state)),
        Redirect::to(url.as_str()),
    ))
}

pub async fn discord_oauth_callback(
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

    let tokens = &discord_utils::get_oauth_tokens(code, &state).await?;
    let me_data = discord_utils::get_user_data(tokens).await?;
    let user_id = me_data.id;
    let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
    discord_utils::store_discord_tokens(
        &user_id,
        &DiscordTokens {
            access_token: tokens.access_token.clone(),
            refresh_token: tokens.refresh_token.clone(),
            expires_at: now + tokens.expires_in * 1000,
            expires_in: tokens.expires_in,
        },
    )
    .await?;
    Ok(StatusCode::OK)
}
