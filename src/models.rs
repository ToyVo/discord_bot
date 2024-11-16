use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
pub struct GameStatus {
    pub game: String,
    pub discord_message_id: String,
}
