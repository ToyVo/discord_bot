use anyhow::Context;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::Json;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::SignedCookieJar;
use serde_json::json;
use serenity::all::Interaction;
use serenity::builder::CreateInteractionResponse;
use serenity::json;
use std::collections::HashMap;

use crate::discord_utils;
use crate::discord_utils::DiscordTokens;
use crate::error::AppError;
use crate::handlers::handle_slash_command;
use crate::routes::AppState;

pub async fn interactions(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Request received: {headers:?} {body}");

    // Parse request body and verifies incoming requests
    if discord_utils::verify_request(&headers, &body, &state)
        .await
        .is_err()
    {
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
