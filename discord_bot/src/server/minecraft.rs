use crate::error::AppError;
use crate::server::{
    discord,
    models::{DiscordMessage, GamePlayers},
    players, AppState,
};
use chrono::Utc;
use tokio::net::TcpStream;
use crate::server::models::{DBCollection, GameServer, MessageType};

async fn track_generic<S: AsRef<str>>(
    minecraft_address: S,
    channel_id: S,
    server: GameServer,
    state: &AppState,
) -> Result<(), AppError> {
    tracing::debug!("Checking players connected to {server}");
    let last_player_names: Option<GamePlayers> = state.db.select((DBCollection::Players.to_string(), server.to_string())).await?;
    let last_player_names = if let Some(data) = last_player_names {
        data.players
    } else {
        vec![]
    };

    let last_message: Option<DiscordMessage> = state.db.select((DBCollection::DiscordMessages.to_string(), server.to_string())).await?;

    if let Err(e) = TcpStream::connect(minecraft_address.as_ref()).await {
        if let Some(message) = last_message {
            if message.message_type == MessageType::PlayerUpdate {
                discord::send_message(&format!("{server} is not running"), MessageType::ServerDown, server.clone(), channel_id, state).await?;
            }
        } else {
            discord::send_message(&format!("{server} is not running"), MessageType::ServerDown, server.clone(), channel_id, state).await?;
        }
        tracing::debug!("{server} unreachable {e}");
        return Ok(());
    }

    let (host, port) = minecraft_address.as_ref().split_once(":")
        .expect("Couldn't separate host and port from minecraft address");
    let port = port.parse::<u16>().expect("couldn't parse port as int");

    let players = if let Some(sample) = mc_query::status(host, port).await?.players.sample {
        sample.iter().map(|player| player.name.clone()).collect()
    } else {
        Vec::new()
    };

    if let Some(message) = players::get_player_changes(&last_player_names, &players) {
        discord::send_message(&message, MessageType::PlayerUpdate, server.clone(), channel_id, state).await?;
    } else {
        if let Some(message) = last_message {
            if message.message_type == MessageType::ServerDown {
                discord::send_message(&format!("No one is connected to {server}"), MessageType::PlayerUpdate, server.clone(), channel_id, state).await?;
            }
        } else {
            discord::send_message(&format!("No one is connected to {server}"), MessageType::PlayerUpdate, server.clone(), channel_id, state).await?;
        }
    }

    let _: Option<GamePlayers> = state
        .db
        .upsert((DBCollection::Players.to_string(), server.to_string()))
        .content(GamePlayers {
            game: server,
            players,
            time: Utc::now(),
        })
        .await?;

    Ok(())
}

pub async fn track_players(state: &AppState) -> Result<(), AppError> {
    track_generic(
        &state.minecraft_modded_address,
        &state.discord_minecraft_modded_channel_id,
        GameServer::MinecraftModded,
        state,
    )
    .await?;
    track_generic(
        &state.minecraft_geyser_address,
        &state.discord_minecraft_geyser_channel_id,
        GameServer::MinecraftGeyser,
        state,
    )
    .await?;
    Ok(())
}
