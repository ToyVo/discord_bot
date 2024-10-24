mod discord_handlers;
mod minecraft_handler;

use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::Router;
use axum_extra::extract::cookie::Key;
use dioxus::prelude::*;
use std::ops::Deref;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tower_http::services::ServeDir;

use crate::routes::discord_handlers::{discord_oauth_callback, interactions, verify_user};
use crate::routes::minecraft_handler::modpack_info_endpoint;

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
    pub base_url: String,
    pub bot_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub discord_bot_spam_channel_id: String,
    pub discord_minecraft_geyser_channel_id: String,
    pub discord_minecraft_modded_channel_id: String,
    pub discord_terraria_channel_id: String,
    pub forge_api_key: String,
    pub key: Key,
    pub minecraft_geyser_rcon_address: String,
    pub minecraft_geyser_rcon_password: String,
    pub minecraft_modded_rcon_address: String,
    pub minecraft_modded_rcon_password: String,
    pub public_key: String,
    pub tshock_base_url: String,
    pub tshock_token: String,
    pub user_agent: String,
    pub db: Surreal<Db>,
}

pub fn app() -> Router<AppState> {
    Router::new()
        .route("/api/interactions", post(interactions))
        .route("/verify-user", get(verify_user))
        .route("/discord-oauth-callback", get(discord_oauth_callback))
        .route("/terms-of-service", get(terms_of_service_endpoint))
        .route("/privacy-policy", get(privacy_policy_endpoint))
        .route("/minecraft", get(modpack_info_endpoint))
        .route("/", get(app_endpoint))
        .nest_service(
            "/modpack",
            ServeDir::new("modpack").not_found_service(get(not_found_endpoint)),
        )
        .nest_service(
            "/public",
            ServeDir::new("public").not_found_service(get(not_found_endpoint)),
        )
        .fallback_service(get(not_found_endpoint))
}

fn html_app<S: AsRef<str>>(content: Element, title: S) -> Html<String> {
    // render the rsx! macro to HTML
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" href="/public/favicon.ico">
    <link rel="apple-touch-icon" sizes="180x180" href="/public/apple-touch-icon.png">
    <link rel="icon" type="image/png" sizes="32x32" href="/public/favicon-32x32.png">
    <link rel="icon" type="image/png" sizes="16x16" href="/public/favicon-16x16.png">
    <link rel="manifest" href="/public/site.webmanifest">
    <title>{}</title>
    <script>0</script>
</head>
{}
</html>"#,
        title.as_ref(),
        dioxus_ssr::render_element(rsx! {
            body {
                width: "100vw",
                height: "100vh",
                margin: "0",
                display: "flex",
                flex_direction: "column",
                font_family: "'Noto Sans', 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif",
                {content}
            }
        })
    ))
}

async fn app_endpoint() -> Html<String> {
    html_app(
        rsx! {
            a {
                href: "/minecraft",
                "Minecraft Modpack"
            }
        },
        "ToyVo's Servers",
    )
}

async fn privacy_policy_endpoint() -> Html<String> {
    html_app(
        rsx! {
            div { "Privacy Policy" }
        },
        "Privacy Policy",
    )
}

async fn terms_of_service_endpoint() -> Html<String> {
    html_app(
        rsx! {
            div { "Terms of Service" }
        },
        "Terms of Service",
    )
}

async fn not_found_endpoint() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        html_app(
            rsx! {
                div { "404" }
            },
            "404",
        ),
    )
}
