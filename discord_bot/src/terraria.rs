use crate::routes::AppState;
use lib::AppError;
use oxford_join::OxfordJoin;
use serde_json::Value;

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
fn get_player_diff(before: &[String], after: &[String]) -> (Vec<String>, Vec<String>) {
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
    let last_player_nicknames = state.terraria_players.read().await;
    let (disconnected, joined) = get_player_diff(&last_player_nicknames, &player_nicknames);

    if !disconnected.is_empty() || !joined.is_empty() {
        // player1, player2, and player3 have joined. player4, player5, and player6 have disconnected
        let message = [
            if !joined.is_empty() {
                format!(
                    "{} {} joined",
                    joined.oxford_join(oxford_join::Conjunction::And),
                    if joined.len() > 1 { "have" } else { "has" }
                )
            } else {
                "".to_string()
            },
            if !disconnected.is_empty() {
                format!(
                    "{} {} disconnected",
                    disconnected.oxford_join(oxford_join::Conjunction::And),
                    if joined.len() > 1 { "have" } else { "has" }
                )
            } else {
                "".to_string()
            },
        ]
        .join(" ");
        tracing::info!("{}", message);
    }

    let mut terraria_players = state.terraria_players.write().await;
    *terraria_players = player_nicknames;
    Ok(())
}
