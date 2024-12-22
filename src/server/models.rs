use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GameBackup {
    pub game: String,
    pub filename: String,
    pub time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct GamePlayers {
    pub game: String,
    pub players: Vec<String>,
    pub time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct DiscordMessage {
    pub game: String,
    pub discord_message_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordTokens<S: AsRef<str>> {
    pub access_token: S,
    pub refresh_token: S,
    pub expires_at: DateTime<Utc>,
    pub expires_in: u64,
}
