use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppState {
    pub minecraft_geyser_address: String,
    pub minecraft_modded_address: String,
    pub terraria_address: String,
    pub tshock_base_url: String,
    pub tshock_token: String,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        h1 { "game manager" }
    }
}

#[get("/api/minecraft/status")]
async fn minecraft_status() -> Result<Value, ServerFnError> {
    Ok(json!({
        "code": 200,
        "status": "ok"
    }))
}

#[get("/api/minecraft/players")]
async fn minecraft_players() -> Result<Value, ServerFnError> {
    // if let Err(e) = TcpStream::connect(minecraft_address.as_ref()).await {
    //     if let Some(message) = last_message {
    //         if message.message_type == MessageType::PlayerUpdate {
    //             discord::send_message(&format!("{server} is not running"), MessageType::ServerDown, server.clone(), channel_id, state).await?;
    //         }
    //     } else if last_message.is_none() {
    //         discord::send_message(&format!("{server} is not running"), MessageType::ServerDown, server.clone(), channel_id, state).await?;
    //     }
    //     tracing::debug!("{server} unreachable {e}");
    //     return Ok(());
    // }

    // let (host, port) = minecraft_address.as_ref().split_once(":")
    //     .expect("Couldn't separate host and port from minecraft address");
    // let port = port.parse::<u16>().expect("couldn't parse port as int");

    // let players = if let Some(sample) = mc_query::status(host, port).await?.players.sample {
    //     sample.iter().map(|player| player.name.clone()).collect()
    // } else {
    //     Vec::new()
    // };
    Ok(json!({
        "code": 200,
        "status": "ok"
    }))
}

#[get("/api/terraria/status")]
/// ref: https://tshock.readme.io/reference/v2status
async fn terraria_status() -> Result<Value, ServerFnError> {
    // TODO use tshock url from state
    let url = format!("{}/v2/server/status?players=true", "http://0.0.0.0:7878");
    let res = reqwest::get(url).await;
    if let Err(e) = res {
        return Err(ServerFnError::new(e));
    }
    let response = res.unwrap().error_for_status();
    if let Err(e) = response {
        return Err(ServerFnError::new(e));
    }
    match response.unwrap().json::<Value>().await {
        Ok(data) => Ok(data),
        Err(e) => Err(ServerFnError::new(e)),
    }
}

#[get("/api/terraria/players")]
async fn terraria_players() -> Result<Value, ServerFnError> {
    // if let Err(e) = TcpStream::connect(&state.terraria_address).await {
    //     if let Some(message) = last_message {
    //         if message.message_type == MessageType::PlayerUpdate {
    //             discord::send_message(&"terraria is not running".to_string(), MessageType::ServerDown, GameServer::Terraria, &state.discord_terraria_channel_id, state).await?;
    //         }
    //     } else if last_message.is_none() {
    //         discord::send_message(&"terraria is not running".to_string(), MessageType::ServerDown, GameServer::Terraria, &state.discord_terraria_channel_id, state).await?;
    //     }
    //     tracing::debug!("terraria unreachable {e}");
    //     return Ok(());
    // }

    // let player_nicknames = if let Ok(status) = get_status(state).await {
    //     let players = status
    //         .get("players")
    //         .expect("players not found")
    //         .as_array()
    //         .expect("failed to parse players into array");
    //     players
    //         .iter()
    //         .map(|player| {
    //             player
    //                 .get("nickname")
    //                 .expect("Could not get nickname")
    //                 .as_str()
    //                 .expect("failed to parse nickname as str")
    //                 .to_string()
    //         })
    //         .collect()
    // } else {
    //     tracing::debug!("terraria not running");
    //     // set players to empty if it isn't already
    //     vec![]
    // };
    Ok(json!({
        "code": 200,
        "status": "ok"
    }))
}
