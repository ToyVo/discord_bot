use crate::error::AppError;
use crate::server::{
    discord,
    models::{DiscordMessage, GamePlayers},
    players, AppState,
};
use chrono::Utc;
use serde_json::Value;
use tokio::net::TcpStream;
use crate::server::models::{DBCollection, GameServer, MessageType};

/// ref: https://tshock.readme.io/reference/v2status
pub async fn get_status(state: &AppState) -> Result<Value, AppError> {
    let url = format!("{}/v2/server/status?players=true", state.tshock_base_url);
    let response = reqwest::get(url).await?.error_for_status()?;
    let data = response.json::<Value>().await?;
    Ok(data)
}

pub async fn track_players(state: &AppState) -> Result<(), AppError> {
    tracing::debug!("Checking players connected to terraria");

    let last_message: Option<DiscordMessage> = state.db.select((DBCollection::DiscordMessages.to_string(), GameServer::Terraria.to_string())).await?;

    if let Err(e) = TcpStream::connect(&state.terraria_address).await {
        if let Some(message) = last_message {
            if message.message_type == MessageType::PlayerUpdate {
                discord::send_message(&"terraria is not running".to_string(), MessageType::PlayerUpdate, GameServer::Terraria, &state.discord_terraria_channel_id, state).await?;
            }
        } else if last_message.is_none() {
            discord::send_message(&"terraria is not running".to_string(), MessageType::PlayerUpdate, GameServer::Terraria, &state.discord_terraria_channel_id, state).await?;
        }
        tracing::debug!("terraria unreachable {e}");
        return Ok(());
    }

    let player_nicknames = if let Ok(status) = get_status(state).await {
        let players = status
            .get("players")
            .expect("players not found")
            .as_array()
            .expect("failed to parse players into array");
        players
            .iter()
            .map(|player| {
                player
                    .get("nickname")
                    .expect("Could not get nickname")
                    .as_str()
                    .expect("failed to parse nickname as str")
                    .to_string()
            })
            .collect()
    } else {
        tracing::debug!("terraria not running");
        // set players to empty if it isn't already
        vec![]
    };

    let last_player_nicknames: Option<GamePlayers> =
        state.db.select((DBCollection::Players.to_string(), GameServer::Terraria.to_string())).await?;
    let last_player_nicknames = if let Some(data) = last_player_nicknames {
        data.players
    } else {
        vec![]
    };

    if let Some(message) = players::get_player_changes(&last_player_nicknames, &player_nicknames) {
        discord::send_message(&message, MessageType::PlayerUpdate, GameServer::Terraria, &state.discord_terraria_channel_id, state).await?;
    } else if let Some(message) = last_message {
        if message.message_type == MessageType::ServerDown {
            discord::send_message(&format!("No one is connected to terraria"), MessageType::PlayerUpdate, GameServer::Terraria, &state.discord_terraria_channel_id, state).await?;
        }
    } else if last_message.is_none() {
        discord::send_message(&format!("No one is connected to terraria"), MessageType::PlayerUpdate, GameServer::Terraria, &state.discord_terraria_channel_id, state).await?;
    }

    let _: Option<GamePlayers> = state
        .db
        .upsert((DBCollection::Players.to_string(), GameServer::Terraria.to_string()))
        .content(GamePlayers {
            game: GameServer::Terraria,
            players: player_nicknames,
            time: Utc::now(),
        })
        .await?;

    Ok(())
}
