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
