use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::{Json, Router};
use axum::routing::{get, post};
use serde_json::{json, Value};
use serenity::all::InteractionType;
use serenity::builder::CreateInteractionResponse;
use serenity::json;
use crate::discord_utils::verify_discord_request;
use crate::handlers::handle_slash_command;

pub fn app() -> Router {
    Router::new()
        .route("/", get(|| async { Html("<h1>Hello, World!</h1>") }))
        .route("/api/interactions", post(interactions))
        .route(
            "/verify-user",
            get(|| async { Html("<h1>Verify User</h1>") }),
        )
        .route(
            "/terms-of-service",
            get(|| async { Html("<h1>Terms of Service</h1>") }),
        )
        .route(
            "/privacy-policy",
            get(|| async { Html("<h1>Privacy Policy</h1>") }),
        )
}

pub async fn interactions(headers: HeaderMap, body: String) -> impl IntoResponse {
    println!("Request received: {body}");

    // Parse request body and verifies incoming requests
    // Disable for debugging purposes when receiving requests from the test_server
    #[cfg(not(debug_assertions))]
    if (verify_discord_request(&headers, &body)).await.is_err() {
        return (StatusCode::UNAUTHORIZED, Json(json!({})));
    }

    let payload = match Json::<Value>::from_bytes(body.as_bytes()) {
        Ok(payload) => payload,
        Err(e) => {
            eprintln!("Could not parse body\n{e:#?}");
            return (StatusCode::BAD_REQUEST, Json(json!({})));
        }
    };

    let request_type = payload
        .get("type")
        .and_then(|s| s.as_u64())
        .and_then(|n| InteractionType::try_from(n as u8).ok());

    match request_type {
        Some(InteractionType::Ping) => {
            println!("Received discord ping request, Replying pong");
            (
                StatusCode::OK,
                Json(json::to_value(CreateInteractionResponse::Pong).unwrap()),
            )
        }
        Some(InteractionType::Command) => {
            match handle_slash_command(payload).await {
                Ok(value) => (StatusCode::OK, Json(value)),
                Err(e) => {
                    eprintln!("Slash command error {e:#?}");
                    (StatusCode::OK, Json(json!({})))
                }
            }
        }
        _ => (StatusCode::OK, Json(json!({})))
    }
}
