use crate::routes::AppState;
use lib::AppError;
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
    tracing::info!("players: {:?}", player_nicknames);
    Ok(())
}
