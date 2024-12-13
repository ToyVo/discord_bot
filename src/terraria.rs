use crate::error::AppError;
#[cfg(feature = "watchers")]
use crate::models::{GamePlayers, DiscordMessage};
use crate::routes::AppState;
#[cfg(feature = "db")]
use crate::DB;
#[cfg(feature = "watchers")]
use crate::{discord_utils, systemctl_running};
#[cfg(feature = "watchers")]
use anyhow::Context;
#[cfg(feature = "watchers")]
use chrono::Utc;
use oxford_join::OxfordJoin;
use serde_json::Value;
#[cfg(feature = "watchers")]
use serenity::all::MessageFlags;
#[cfg(feature = "watchers")]
use serenity::builder::CreateMessage;

/// expected structure:
/// not returning anything parsed just in case
/// ```json
/// {
///   "status": "200",
///   "name": "server name",
///   "serverversion": "v1.4.4.9",
///   "tshockversion": "5.2.0.0",
///   "port": 7777,
///   "playercount": 1,
///   "maxplayers": 8,
///   "world": "server name",
///   "uptime": "0.01:30:36",
///   "serverpassword": true,
///   "players": [
///     {
///       "nickname": "player",
///       "username": "",
///       "group": "guest",
///       "active": true,
///       "state": 10,
///       "team": 0
///     }
///   ]
/// }
/// ```
pub async fn get_status(state: &AppState) -> Result<Value, AppError> {
    let url = format!("{}/v2/server/status?players=true", state.tshock_base_url);
    let response = reqwest::get(url).await?.error_for_status()?;
    let data = response.json::<Value>().await?;
    Ok(data)
}

pub async fn broadcast<T: AsRef<str>>(state: &AppState, message: T) -> Result<(), AppError> {
    let url = format!(
        "{}/v2/server/broadcast?msg={}&token={}",
        state.tshock_base_url,
        message.as_ref(),
        state.tshock_token
    );
    reqwest::get(url).await?.error_for_status()?;
    Ok(())
}

/// take two lists of player names and return the difference between them.
/// the first tuple is the list of players who have disconnected. the second tuple is of players who have joined.
pub fn get_player_diff(
    before: &[String],
    after: &[String],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let disconnected = before
        .iter()
        .filter(|&player| !after.contains(player))
        .cloned()
        .collect();
    let mut joined = vec![];
    let mut remaining_online = vec![];
    for player in after {
        if !before.contains(player) {
            joined.push(player.clone());
        } else {
            remaining_online.push(player.clone());
        }
    }

    (disconnected, joined, remaining_online)
}

pub fn get_player_changes(before: &[String], after: &[String]) -> Option<String> {
    let (disconnected, joined, remaining) = get_player_diff(before, after);
    if disconnected.is_empty() && joined.is_empty() {
        return None;
    }
    // player1, player2, and player3 have joined. player4, player5, and player6 have disconnected
    Some(
        [
            if !joined.is_empty() {
                format!(
                    "{} {} joined.",
                    joined.oxford_join(oxford_join::Conjunction::And),
                    if joined.len() != 1 { "have" } else { "has" }
                )
            } else {
                "".to_string()
            },
            if !disconnected.is_empty() {
                format!(
                    "{} {} disconnected.",
                    disconnected.oxford_join(oxford_join::Conjunction::And),
                    if disconnected.len() != 1 {
                        "have"
                    } else {
                        "has"
                    }
                )
            } else {
                "".to_string()
            },
            if !remaining.is_empty() {
                format!(
                    "{} {} online.",
                    remaining.oxford_join(oxford_join::Conjunction::And),
                    if remaining.len() != 1 { "are" } else { "is" }
                )
            } else if !disconnected.is_empty() && joined.is_empty() {
                "Nobody is online.".to_string()
            } else {
                "".to_string()
            },
        ]
        .join(" "),
    )
}

#[cfg(feature = "watchers")]
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

    let last_player_nicknames: Option<GamePlayers> = DB.select(("players", "terraria")).await?;
    let last_player_nicknames = if let Some(data) = last_player_nicknames {
        data.players
    } else {
        vec![]
    };

    if let Some(message) = get_player_changes(&last_player_nicknames, &player_nicknames) {
        tracing::info!("{}", message);
        let message = discord_utils::create_message(
            CreateMessage::new()
                .content(message)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
            &state.discord_terraria_channel_id,
            state,
        )
        .await?;

        match DB.select(("discord_messages", "terraria")).await {
            Ok(Some(data)) => {
                let data: DiscordMessage = data;
                discord_utils::delete_message(
                    &data.discord_message_id,
                    &state.discord_terraria_channel_id,
                    state,
                )
                .await
            }
            Err(e) => Ok(tracing::error!("Error getting DiscordMessage from DB: {}", e)),
            _ => Ok(()),
        }?;

        let _upserted: Option<DiscordMessage> = DB
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

    let _upserted: Option<GamePlayers> = DB
        .upsert(("players", "terraria"))
        .content(GamePlayers {
            game: String::from("terraria"),
            players: player_nicknames,
            time: Utc::now(),
        })
        .await?;

    Ok(())
}
