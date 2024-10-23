use crate::discord_utils;
use crate::error::AppError;
use crate::models::{GamePlayers, GameStatus};
use crate::routes::AppState;
use anyhow::Context;
use oxford_join::OxfordJoin;
use serde_json::{json, Value};

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
pub fn get_player_diff(before: &[String], after: &[String]) -> (Vec<String>, Vec<String>) {
    let disconnected = before
        .iter()
        .filter(|&player| !after.contains(player))
        .cloned()
        .collect();
    let joined = after
        .iter()
        .filter_map(|player| {
            if !before.contains(player) {
                Some(player.clone())
            } else {
                None
            }
        })
        .collect();

    (disconnected, joined)
}

pub fn get_player_changes(before: &[String], after: &[String]) -> Option<String> {
    let (disconnected, joined) = get_player_diff(before, after);
    if disconnected.is_empty() && joined.is_empty() {
        return None;
    }
    // player1, player2, and player3 have joined. player4, player5, and player6 have disconnected
    Some(
        [
            if !joined.is_empty() {
                format!(
                    "{} {} joined",
                    joined.oxford_join(oxford_join::Conjunction::And),
                    if joined.len() != 1 { "have" } else { "has" }
                )
            } else {
                "".to_string()
            },
            if !disconnected.is_empty() {
                format!(
                    "{} {} disconnected",
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
            format!(
                "There {} {} player{} online",
                if after.len() != 1 { "are" } else { "is" },
                after.len(),
                if after.len() != 1 { "s" } else { "" }
            ),
        ]
        .join(" "),
    )
}

pub async fn track_players(state: &AppState) -> Result<(), AppError> {
    // get nicknames
    let status = get_status(state).await?;
    let players = status
        .get("players")
        .expect("players not found")
        .as_array()
        .expect("failed to parse players into array");
    let player_nicknames: Vec<String> = players
        .iter()
        .map(|player| {
            player
                .get("nickname")
                .expect("Could not get nickname")
                .as_str()
                .expect("failed to parse nickname as str")
                .to_string()
        })
        .collect();

    let last_player_nicknames: Option<GamePlayers> =
        state.db.select(("players", "terraria")).await?;
    let last_player_nicknames = if let Some(data) = last_player_nicknames {
        data.players
    } else {
        vec![]
    };

    if let Some(message) = get_player_changes(&last_player_nicknames, &player_nicknames) {
        tracing::info!("{}", message);
        let message = discord_utils::create_message(
            json!({
                "content": message,
            }),
            &state.discord_terraria_channel_id,
            state,
        )
        .await?;

        let last_message_id: Option<GameStatus> = state.db.select(("status", "terraria")).await?;
        if let Some(data) = last_message_id {
            discord_utils::delete_message(
                &data.discord_message_id,
                &state.discord_terraria_channel_id,
                state,
            )
            .await?;
        }

        let _upserted: Option<GameStatus> = state
            .db
            .upsert(("status", "terraria"))
            .content(GameStatus {
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

    let _upserted: Option<GamePlayers> = state
        .db
        .upsert(("players", "terraria"))
        .content(GamePlayers {
            game: String::from("terraria"),
            players: player_nicknames,
        })
        .await?;

    Ok(())
}
