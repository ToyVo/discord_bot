use anyhow::Context;
use chrono::Utc;
use serde_json::Value;
use serenity::all::MessageFlags;
use serenity::builder::CreateMessage;
use crate::error::AppError;
use crate::server::{AppState, discord, players, models::{DiscordMessage, GamePlayers}};

/// ref: https://tshock.readme.io/reference/v2status
pub async fn get_status(state: &AppState) -> Result<Value, AppError> {
    let url = format!("{}/v2/server/status?players=true", state.tshock_base_url);
    let response = reqwest::get(url).await?.error_for_status()?;
    let data = response.json::<Value>().await?;
    Ok(data)
}

pub async fn track_players(state: &AppState) -> Result<(), AppError> {
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

    let last_player_nicknames: Option<GamePlayers> = state.db.select(("players", "terraria")).await?;
    let last_player_nicknames = if let Some(data) = last_player_nicknames {
        data.players
    } else {
        vec![]
    };

    if let Some(message) = players::get_player_changes(&last_player_nicknames, &player_nicknames) {
        tracing::info!("{}", message);
        let message = discord::create_message(
            CreateMessage::new()
                .content(message)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
            &state.discord_terraria_channel_id,
            state,
        )
        .await?;

        match state.db.select(("discord_messages", "terraria")).await {
            Ok(Some(data)) => {
                let data: DiscordMessage = data;
                discord::delete_message(
                    &data.discord_message_id,
                    &state.discord_terraria_channel_id,
                    state,
                )
                .await
            }
            Err(e) => Ok(tracing::error!("Error getting DiscordMessage from DB: {}", e)),
            _ => Ok(()),
        }?;

        let _upserted: Option<DiscordMessage> = state.db
            .upsert(("discord_messages", "terraria"))
            .content(DiscordMessage {
                game: String::from("terraria"),
                discord_message_id: message
                    .get("id")
                    .context("Could not find id in response")?
                    .as_str()
                    .context("could not parse as str")?
                    .to_string(),
            })
            .await?;
    }

    let _upserted: Option<GamePlayers> = state.db
        .upsert(("players", "terraria"))
        .content(GamePlayers {
            game: String::from("terraria"),
            players: player_nicknames,
            time: Utc::now(),
        })
        .await?;

    Ok(())
}
