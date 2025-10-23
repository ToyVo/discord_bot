use {
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    std::fmt::Formatter,
};

#[derive(Serialize, Deserialize)]
pub struct GamePlayers {
    pub game: GameServer,
    pub players: Vec<String>,
    pub time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct DiscordMessage {
    pub game: GameServer,
    pub discord_message_id: String,
    pub message_type: MessageType,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    ServerDown,
    PlayerUpdate,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum GameServer {
    MinecraftModded,
    MinecraftGeyser,
    Terraria,
}

impl std::fmt::Display for GameServer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MinecraftGeyser => write!(f, "minecraft_geyser"),
            Self::MinecraftModded => write!(f, "minecraft_modded"),
            Self::Terraria => write!(f, "terraria"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum DBCollection {
    DiscordMessages,
    Players,
}

impl std::fmt::Display for DBCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DiscordMessages => write!(f, "discord_messages"),
            Self::Players => write!(f, "players"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordTokens<S: AsRef<str>> {
    pub access_token: S,
    pub refresh_token: S,
    pub expires_at: DateTime<Utc>,
    pub expires_in: u64,
}
