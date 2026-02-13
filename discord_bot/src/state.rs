use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "server")]
use {crate::error::AppError, std::sync::Arc, tokio::sync::Mutex};

// Global state - this will be set by the main function
#[cfg(feature = "server")]
pub static GLOBAL_STATE: std::sync::OnceLock<Arc<Mutex<AppState>>> = std::sync::OnceLock::new();

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum MessageType {
    RoleAssigner,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppState {
    pub base_url: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_public_key: String,
    pub discord_token: String,
    pub user_agent: String,
}

#[cfg(feature = "server")]
pub fn set_global_state(state: Arc<Mutex<AppState>>) {
    let _ = GLOBAL_STATE.set(state);
}

#[cfg(feature = "server")]
pub type Context<'a> = poise::Context<'a, Arc<Mutex<AppState>>, AppError>;
