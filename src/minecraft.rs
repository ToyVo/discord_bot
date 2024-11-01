#[cfg(feature = "watchers")]
use crate::error::AppError;
#[cfg(feature = "watchers")]
use crate::models::{GamePlayers, GameStatus};
#[cfg(feature = "watchers")]
use crate::routes::AppState;
#[cfg(feature = "watchers")]
use crate::terraria::get_player_changes;
#[cfg(feature = "watchers")]
use crate::{discord_utils, systemctl_running};
#[cfg(feature = "watchers")]
use anyhow::Context;
#[cfg(feature = "watchers")]
use serde_json::json;

#[cfg(feature = "watchers")]
async fn track_generic<S: AsRef<str>>(
    minecraft_rcon_address: S,
    minecraft_rcon_password: S,
    discord_minecraft_channel_id: S,
    surreal_id: &str,
    service_name: &str,
    state: &AppState,
) -> Result<(), AppError> {
    if !systemctl_running(service_name).await? {
        return Ok(());
    }

    let mut server = <rcon::Connection<tokio::net::TcpStream>>::builder()
        .enable_minecraft_quirks(true)
        .connect(
            minecraft_rcon_address.as_ref(),
            minecraft_rcon_password.as_ref(),
        )
        .await?;

    // list response "There are n of a max of m players online: <player1>"
    let res = server.cmd("list").await?;

    // Parse response to get list of player names in a vector
    let start_index = res.find(':').context("Could not find ':' in response")?;
    let players = res[start_index + 1..]
        .trim()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();

    let last_player_names: Option<GamePlayers> = state.db.select(("players", surreal_id)).await?;
    let last_player_names = if let Some(data) = last_player_names {
        data.players
    } else {
        vec![]
    };

    if let Some(message) = get_player_changes(&last_player_names, &players) {
        tracing::info!("{}", message);
        let message = discord_utils::create_message(
            json!({
                "content": message
            }),
            &discord_minecraft_channel_id,
            state,
        )
        .await?;

        match state.db.select(("status", surreal_id)).await {
            Ok(Some(data)) => {
                let data: GameStatus = data;
                discord_utils::delete_message(
                    data.discord_message_id.as_str(),
                    discord_minecraft_channel_id.as_ref(),
                    state,
                )
                .await
            }
            Err(e) => Ok(tracing::error!("Error getting GameStatus from DB: {}", e)),
            _ => Ok(()),
        }?;

        let _upserted: Option<GameStatus> = state
            .db
            .upsert(("status", surreal_id))
            .content(GameStatus {
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

    let _upserted: Option<GamePlayers> = state
        .db
        .upsert(("players", surreal_id))
        .content(GamePlayers {
            game: surreal_id.to_string(),
            players,
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
        &state.minecraft_modded_service_name,
        state,
    )
    .await?;
    track_generic(
        &state.minecraft_geyser_rcon_address,
        &state.minecraft_geyser_rcon_password,
        &state.discord_minecraft_geyser_channel_id,
        "minecraft_geyser",
        &state.minecraft_geyser_service_name,
        state,
    )
    .await?;
    Ok(())
}
