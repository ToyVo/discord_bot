#[cfg(feature = "watchers")]
use crate::discord_utils;
#[cfg(feature = "watchers")]
use crate::error::AppError;
#[cfg(feature = "watchers")]
use crate::models::{GamePlayers, DiscordMessage};
#[cfg(feature = "watchers")]
use crate::routes::AppState;
#[cfg(feature = "watchers")]
use crate::terraria::get_player_changes;
#[cfg(feature = "db")]
use crate::DB;
#[cfg(feature = "watchers")]
use anyhow::Context;
#[cfg(feature = "watchers")]
use chrono::Utc;
#[cfg(feature = "watchers")]
use rcon::Connection;
#[cfg(feature = "watchers")]
use serenity::all::MessageFlags;
#[cfg(feature = "watchers")]
use serenity::builder::CreateMessage;
#[cfg(feature = "watchers")]
use tokio::net::TcpStream;

#[cfg(feature = "watchers")]
async fn track_generic<S: AsRef<str>>(
    minecraft_rcon_address: S,
    minecraft_rcon_password: S,
    discord_minecraft_channel_id: S,
    surreal_id: &str,
    state: &AppState,
) -> Result<(), AppError> {
    let last_player_names: Option<GamePlayers> = DB.select(("players", surreal_id)).await?;
    let last_player_names = if let Some(data) = last_player_names {
        data.players
    } else {
        vec![]
    };

    let players = if let Ok(mut connection) = <Connection<TcpStream>>::builder()
        .enable_minecraft_quirks(true)
        .connect(
            minecraft_rcon_address.as_ref(),
            minecraft_rcon_password.as_ref(),
        )
        .await
    {
        // list response "There are n of a max of m players online: <player1>"
        let res = connection.cmd("list").await?;

        // Parse response to get list of player names in a vector
        let start_index = res.find(':').context("Could not find ':' in response")?;
        res[start_index + 1..]
            .trim()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>()
    } else {
        tracing::debug!("mc not running");
        vec![]
    };

    if let Some(message) = get_player_changes(&last_player_names, &players) {
        tracing::info!("{}", message);
        let message = discord_utils::create_message(
            CreateMessage::new()
                .content(message)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
            &discord_minecraft_channel_id,
            state,
        )
        .await?;

        match DB.select(("discord_messages", surreal_id)).await {
            Ok(Some(data)) => {
                let data: DiscordMessage = data;
                discord_utils::delete_message(
                    data.discord_message_id.as_str(),
                    discord_minecraft_channel_id.as_ref(),
                    state,
                )
                .await
            }
            Err(e) => Ok(tracing::error!("Error getting DiscordMessage from DB: {}", e)),
            _ => Ok(()),
        }?;

        // TODO: if the last message was sent within the last 5 minutes, just update the message instead of creating a new one
        // if t
        let _upserted: Option<DiscordMessage> = DB
            .upsert(("discord_messages", surreal_id))
            .content(DiscordMessage {
                game: surreal_id.to_string(),
                discord_message_id: message
                    .get("id")
                    .context("Could not find id in response")?
                    .as_str()
                    .context("could not parse as str")?
                    .to_string(),
            })
            .await?;
    }

    let _upserted: Option<GamePlayers> = DB
        .upsert(("players", surreal_id))
        .content(GamePlayers {
            game: surreal_id.to_string(),
            players,
            time: Utc::now(),
        })
        .await?;

    Ok(())
}

#[cfg(feature = "watchers")]
pub async fn track_players(state: &AppState) -> Result<(), AppError> {
    track_generic(
        &state.minecraft_modded_rcon_address,
        &state.minecraft_modded_rcon_password,
        &state.discord_minecraft_modded_channel_id,
        "minecraft_modded",
        state,
    )
    .await?;
    track_generic(
        &state.minecraft_geyser_rcon_address,
        &state.minecraft_geyser_rcon_password,
        &state.discord_minecraft_geyser_channel_id,
        "minecraft_geyser",
        state,
    )
    .await?;
    Ok(())
}
