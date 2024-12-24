use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use std::ops::Deref;
use std::sync::Arc;

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
    pub client_id: String,
    pub client_secret: String,
    pub cloud_ssh_host: Option<String>,
    pub discord_bot_spam_channel_id: String,
    pub discord_minecraft_geyser_channel_id: String,
    pub discord_minecraft_modded_channel_id: String,
    pub discord_terraria_channel_id: String,
    pub discord_token: String,
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
    pub db: surrealdb::Surreal<surrealdb::engine::local::Db>,
}
