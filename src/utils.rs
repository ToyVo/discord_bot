use crate::commands::Command;
use rand::seq::SliceRandom;
use reqwest::{Error, Method, Response};
use serde::Serialize;
use std::env::var;

pub async fn discord_request<S: AsRef<str>, T: Serialize + ?Sized>(
    endpoint: S,
    method: Method,
    body: Option<&T>,
) -> Result<Response, Error> {
    let bot_token = var("DISCORD_BOT_TOKEN").unwrap_or_default();
    let repo = var("CARGO_PKG_REPOSITORY").unwrap_or_default();
    let version = var("CARGO_PKG_VERSION").unwrap_or_default();
    // append endpoint to root API URL
    let url = format!("https://discord.com/api/v10/{}", endpoint.as_ref());
    let client = reqwest::Client::new();
    let mut builder = client
        .request(method, url)
        .header("Authorization", format!("Bot {bot_token}"))
        .header("User-Agent", format!("DiscordBot ({repo}, {version})"));
    if let Some(b) = body {
        builder = builder
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(b);
    }
    builder.send().await?.error_for_status()
}

pub async fn install_global_commands(commands: Vec<Command<String>>) -> Result<Response, Error> {
    let app_id = var("DISCORD_CLIENT_ID").unwrap_or_default();
    // API endpoint to overwrite global commands
    let endpoint = format!("applications/{app_id}/commands");
    // This is calling the bulk overwrite endpoint: https://discord.com/developers/docs/interactions/application-commands#bulk-overwrite-global-application-commands
    discord_request(endpoint, Method::PUT, Some(&commands)).await
}

// Simple method that returns a random emoji from list
pub fn get_random_emoji() -> &'static str {
    const EMOJI_LIST: &[&str] = &[
        "😭",
        "😄",
        "😌",
        "🤓",
        "😎",
        "😤",
        "🤖",
        "😶‍🌫️",
        "🌏",
        "📸",
        "💿",
        "👋",
        "🌊",
        "✨",
    ];
    EMOJI_LIST.choose(&mut rand::thread_rng()).unwrap()
}

pub fn capitalize<S: AsRef<str>>(s: S) -> String {
    let mut chars = s.as_ref().chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
