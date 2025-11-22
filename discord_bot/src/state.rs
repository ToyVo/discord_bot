use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "server")]
use {crate::error::AppError, std::sync::Arc, tokio::sync::Mutex};

// Global state - this will be set by the main function
#[cfg(feature = "server")]
pub static GLOBAL_STATE: std::sync::OnceLock<Arc<Mutex<AppState>>> = std::sync::OnceLock::new();

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum MessageType {
    RoleAssigner,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppState {
    pub base_url: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_public_key: String,
    pub discord_token: String,
    pub user_agent: String,
    pub message_ids: HashMap<u64, MessageType>,
    pub self_assignable_roles: HashMap<String, u64>,
}

#[cfg(feature = "server")]
pub fn set_global_state(state: Arc<Mutex<AppState>>) {
    let _ = GLOBAL_STATE.set(state);
}

#[cfg(feature = "server")]
pub type Context<'a> = poise::Context<'a, Arc<Mutex<AppState>>, AppError>;
