use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use {crate::error::AppError, std::sync::Arc, tokio::sync::Mutex};

// Global state - this will be set by the main function
#[cfg(feature = "server")]
pub static GLOBAL_STATE: std::sync::OnceLock<Arc<Mutex<AppState>>> = std::sync::OnceLock::new();

#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppState {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub discord_bot_spam_channel_id: String,
    pub discord_minecraft_geyser_channel_id: String,
    pub discord_minecraft_modded_channel_id: String,
    pub discord_terraria_channel_id: String,
    pub discord_token: String,
    pub forge_api_key: String,
    pub minecraft_geyser_address: String,
    pub minecraft_modded_address: String,
    pub public_key: String,
    pub terraria_address: String,
    pub tshock_base_url: String,
    pub tshock_token: String,
    pub user_agent: String,
}

#[cfg(feature = "server")]
pub fn set_global_state(state: Arc<Mutex<AppState>>) {
    let _ = GLOBAL_STATE.set(state);
}

#[cfg(feature = "server")]
pub type Context<'a> = poise::Context<'a, Arc<Mutex<AppState>>, AppError>;
