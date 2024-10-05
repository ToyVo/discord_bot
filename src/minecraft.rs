use crate::discord_utils;
use crate::error::AppError;
use crate::routes::AppState;
use crate::terraria::get_player_changes;
use anyhow::Context;
use serde_json::json;
use tokio::sync::RwLock;

async fn track_generic<S: AsRef<str>>(
    minecraft_rcon_address: S,
    minecraft_rcon_password: S,
    minecraft_players: &RwLock<Vec<String>>,
    discord_minecraft_channel_id: S,
    discord_minecraft_last_message_id: &RwLock<Option<String>>,
    state: &AppState,
) -> Result<(), AppError> {
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

    {
        let last_player_names = minecraft_players.read().await;
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

            {
                let last_message_id = discord_minecraft_last_message_id.read().await;
                if let Some(id) = last_message_id.as_ref() {
                    discord_utils::delete_message(
                        id.as_str(),
                        discord_minecraft_channel_id.as_ref(),
                        state,
                    )
                    .await?;
                }
            }

            let mut discord_minecraft_last_message_id =
                discord_minecraft_last_message_id.write().await;
            *discord_minecraft_last_message_id = Some(
                message
                    .get("id")
                    .context("Could not find id in response")?
                    .as_str()
                    .context("could not parse as str")?
                    .to_string(),
            );
        }
    }

    let mut minecraft_players = minecraft_players.write().await;
    *minecraft_players = players;
    Ok(())
}

pub async fn track_players(state: &AppState) -> Result<(), AppError> {
    track_generic(
        &state.minecraft_rcon_address,
        &state.minecraft_rcon_password,
        &state.minecraft_players,
        &state.discord_minecraft_channel_id,
        &state.discord_minecraft_last_message_id,
        state,
    )
    .await?;
    track_generic(
        &state.minecraft_geyser_rcon_address,
        &state.minecraft_geyser_rcon_password,
        &state.minecraft_geyser_players,
        &state.discord_minecraft_geyser_channel_id,
        &state.discord_minecraft_geyser_last_message_id,
        state,
    )
    .await?;
    Ok(())
}
