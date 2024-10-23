use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GamePlayers {
    pub game: String,
    pub players: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GameStatus {
    pub game: String,
    pub discord_message_id: String,
}
