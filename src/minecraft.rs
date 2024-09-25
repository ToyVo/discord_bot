use crate::discord_utils;
use crate::error::AppError;
use crate::routes::AppState;
use crate::terraria::get_player_changes;
use anyhow::Context;
use serde_json::json;

pub async fn track_players(state: &AppState) -> Result<(), AppError> {
    let mut server = <rcon::Connection<tokio::net::TcpStream>>::builder()
        .enable_minecraft_quirks(true)
        .connect(
            state.minecraft_rcon_address.as_str(),
            state.minecraft_rcon_password.as_str(),
        )
        .await?;

    // list response "There are n of a max of m players online: <player1>"
    // TODO: determine how many players are delimited in the response, assuming comma for now
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
        let last_player_names = state.minecraft_players.read().await;
        if let Some(message) = get_player_changes(&last_player_names, &players) {
            tracing::info!("{}", message);
            let message = discord_utils::create_message(
                json!({
                    "content": message
                }),
                &state.discord_bot_spam_channel_id,
                state,
            )
            .await?;

            {
                let last_message_id = state.discord_minecraft_last_message_id.read().await;
                if let Some(id) = last_message_id.as_ref() {
                    discord_utils::delete_message(id, &state.discord_bot_spam_channel_id, state)
                        .await?;
                }
            }

            let mut discord_minecraft_last_message_id =
                state.discord_minecraft_last_message_id.write().await;
            *discord_minecraft_last_message_id = Some(
                message
                    .get("id")
                    .context("Could not find id in response")?
                    .to_string(),
            );
        }
    }

    let mut minecraft_players = state.minecraft_players.write().await;
    *minecraft_players = players;
    Ok(())
}
