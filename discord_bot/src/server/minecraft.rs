use crate::error::AppError;
use crate::server::{
    discord,
    models::{DiscordMessage, GamePlayers},
    players, AppState,
};
use anyhow::Context;
use chrono::Utc;
use serenity::all::MessageFlags;
use serenity::builder::CreateMessage;
use tokio::net::TcpStream;

async fn track_generic<S: AsRef<str>>(
    minecraft_address: S,
    discord_minecraft_channel_id: S,
    surreal_id: &str,
    state: &AppState,
) -> Result<(), AppError> {
    let last_player_names: Option<GamePlayers> = state.db.select(("players", surreal_id)).await?;
    let last_player_names = if let Some(data) = last_player_names {
        data.players
    } else {
        vec![]
    };

    if let Err(e) = TcpStream::connect(minecraft_address.as_ref()).await {
        tracing::debug!("{surreal_id} unreachable {e}");
        return Ok(());
    }
    
    let (host, port) = minecraft_address.as_ref().split_once(":")
        .expect("Couldn't separate host and port from minecraft address");
    let port = port.parse::<u16>().expect("couldn't parse port as int");

    let data = mc_query::status(host, port).await?;
    let players = if let Some(sample) = data.players.sample {
        sample.iter().map(|player| player.name.clone()).collect()
    } else {
        tracing::debug!("{surreal_id} not running");
        Vec::new()
    };

    if let Some(message) = players::get_player_changes(&last_player_names, &players) {
        tracing::info!("{}", message);
        let message = discord::create_message(
            CreateMessage::new()
                .content(message)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
            &discord_minecraft_channel_id,
            state,
        )
        .await?;

        match state.db.select(("discord_messages", surreal_id)).await {
            Ok(Some(data)) => {
                let data: DiscordMessage = data;
                discord::delete_message(
                    data.discord_message_id.as_str(),
                    discord_minecraft_channel_id.as_ref(),
                    state,
                )
                .await
            }
            Err(e) => {
                tracing::error!("Error getting DiscordMessage from DB: {}", e);
                Ok(())
            }
            _ => Ok(()),
        }?;

        // TODO: if the last message was sent within the last 5 minutes, just update the message instead of creating a new one
        // if t
        let _upserted: Option<DiscordMessage> = state
            .db
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

    let _upserted: Option<GamePlayers> = state
        .db
        .upsert(("players", surreal_id))
        .content(GamePlayers {
            game: surreal_id.to_string(),
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
        "minecraft_modded",
        state,
    )
    .await?;
    track_generic(
        &state.minecraft_geyser_address,
        &state.discord_minecraft_geyser_channel_id,
        "minecraft_geyser",
        state,
    )
    .await?;
    Ok(())
}
