#[cfg(feature = "watchers")]
use crate::error::AppError;
#[cfg(feature = "backups")]
use crate::fs_sync;
#[cfg(feature = "backups")]
use crate::models::GameBackup;
#[cfg(feature = "watchers")]
use crate::models::{GamePlayers, GameStatus};
#[cfg(feature = "backups")]
use crate::rclone;
#[cfg(feature = "watchers")]
use crate::routes::AppState;
#[cfg(feature = "watchers")]
use crate::terraria::get_player_changes;
#[cfg(feature = "db")]
use crate::DB;
#[cfg(feature = "watchers")]
use crate::{discord_utils, systemctl_running};
#[cfg(feature = "watchers")]
use anyhow::Context;
#[cfg(feature = "backups")]
use chrono::NaiveDateTime;
#[cfg(feature = "watchers")]
use chrono::Utc;
#[cfg(feature = "watchers")]
use rcon::Connection;
#[cfg(feature = "watchers")]
use serenity::all::MessageFlags;
#[cfg(feature = "watchers")]
use serenity::builder::CreateMessage;
#[cfg(feature = "backups")]
use std::fs;
#[cfg(feature = "backups")]
use std::time::Duration;
#[cfg(feature = "watchers")]
use tokio::net::TcpStream;
#[cfg(feature = "backups")]
use tokio::process::Command;
#[cfg(feature = "watchers")]
use tokio::sync::RwLock;

#[cfg(feature = "watchers")]
async fn initiate_connection<S: AsRef<str>>(
    minecraft_rcon_address: S,
    minecraft_rcon_password: S,
    service_name: &str,
    connection: &RwLock<Option<Connection<TcpStream>>>,
) -> Result<bool, AppError> {
    let mut con = connection.write().await;

    if !systemctl_running(service_name).await? {
        if con.is_some() {
            *con = None;
        }
        return Ok(false);
    }

    if con.is_none() {
        let server = <Connection<TcpStream>>::builder()
            .enable_minecraft_quirks(true)
            .connect(
                minecraft_rcon_address.as_ref(),
                minecraft_rcon_password.as_ref(),
            )
            .await?;
        *con = Some(server);
    }
    Ok(true)
}

#[cfg(feature = "watchers")]
async fn track_generic<S: AsRef<str>>(
    minecraft_rcon_address: S,
    minecraft_rcon_password: S,
    discord_minecraft_channel_id: S,
    surreal_id: &str,
    service_name: &str,
    connection: &RwLock<Option<Connection<TcpStream>>>,
    state: &AppState,
) -> Result<(), AppError> {
    let last_player_names: Option<GamePlayers> = DB.select(("players", surreal_id)).await?;
    let last_player_names = if let Some(data) = last_player_names {
        data.players
    } else {
        vec![]
    };

    if !initiate_connection(
        minecraft_rcon_address,
        minecraft_rcon_password,
        service_name,
        connection,
    )
    .await?
    {
        if !last_player_names.is_empty() {
            let _upserted: Option<GamePlayers> = DB
                .upsert(("players", surreal_id))
                .content(GamePlayers {
                    game: surreal_id.to_string(),
                    players: vec![],
                    time: Utc::now(),
                })
                .await?;
        }
        return Ok(());
    }

    let mut con = connection.write().await;
    let server = con.as_mut().unwrap();

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

    if let Some(message) = get_player_changes(&last_player_names, &players) {
        tracing::info!("{}", message);
        let message = discord_utils::create_message(
            CreateMessage::new()
                .content(message)
                .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
            &discord_minecraft_channel_id,
            state,
        )
        .await?;

        match DB.select(("status", surreal_id)).await {
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

        let _upserted: Option<GameStatus> = DB
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

    let _upserted: Option<GamePlayers> = DB
        .upsert(("players", surreal_id))
        .content(GamePlayers {
            game: surreal_id.to_string(),
            players,
            time: Utc::now(),
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
        &state.minecraft_modded_connection,
        state,
    )
    .await?;
    track_generic(
        &state.minecraft_geyser_rcon_address,
        &state.minecraft_geyser_rcon_password,
        &state.discord_minecraft_geyser_channel_id,
        "minecraft_geyser",
        &state.minecraft_geyser_service_name,
        &state.minecraft_geyser_connection,
        state,
    )
    .await?;
    Ok(())
}

#[cfg(feature = "backups")]
pub async fn backup_data_dir<S: AsRef<str>>(
    minecraft_rcon_address: S,
    minecraft_rcon_password: S,
    minecraft_data_dir: String,
    surreal_id: &str,
    service_name: &str,
    connection: &RwLock<Option<Connection<TcpStream>>>,
    state: &AppState,
) -> Result<(), AppError> {
    let now = Utc::now();
    let backup_interval = Duration::from_secs(state.backup_interval);

    let last_backup_time = match DB.select(("last_backup", surreal_id)).await {
        Ok(Some(data)) => data,
        _ => GameBackup {
            game: surreal_id.to_string(),
            // ensure a backup is created
            time: now - backup_interval,
            filename: "".to_string(),
        },
    };

    let last_player_names = match DB.select(("players", surreal_id)).await {
        Ok(Some(data)) => data,
        _ => GamePlayers {
            game: surreal_id.to_string(),
            // ensure a backup is created
            time: now - backup_interval,
            players: vec![],
        },
    };

    let should_backup_if_no_players = last_backup_time.time <= last_player_names.time;

    if last_backup_time.time + backup_interval <= now
        && (!last_player_names.players.is_empty() || should_backup_if_no_players)
    {
        let server_running = initiate_connection(
            minecraft_rcon_address,
            minecraft_rcon_password,
            service_name,
            connection,
        )
        .await?;

        let mut con = connection.write().await;

        if server_running {
            let server = con.as_mut().unwrap();

            let _ = server.cmd("save-off").await;
            let _ = server.cmd("save-all flush").await;
            fs_sync().await?;
        }

        let mut tar_args = vec![
            String::from("-C"),
            minecraft_data_dir.clone(),
            String::from("--zstd"),
            String::from("-cf"),
        ];
        let backup_name = format!("{surreal_id}-{}.tar.zst", now.format("%Y-%m-%d_%H-%M-%S"));
        let backup_destination = format!("{}/backups/{backup_name}", minecraft_data_dir.clone());
        tar_args.push(backup_destination.clone());
        let exclude_patterns = vec![
            "*.jar",
            "cache",
            "logs",
            "*.tmp",
            "backups",
            "server.properties",
        ];
        for pattern in exclude_patterns {
            let arg = format!("--exclude={pattern}");
            tar_args.push(arg);
        }
        tar_args.push(String::from("."));

        let local_backup_dir = format!("{}/backups", minecraft_data_dir);
        fs::create_dir_all(&local_backup_dir)?;

        let output = Command::new("tar")
            // add --exclude PATTERN as many times as needed
            .args(&tar_args)
            .output()
            .await?;

        tracing::debug!("tar {}", output.status);

        if server_running {
            let server = con.as_mut().unwrap();

            let _ = server.cmd("save-on").await;
        }

        let _upserted: Option<GameBackup> = DB
            .upsert(("last_backup", surreal_id))
            .content(GameBackup {
                game: surreal_id.to_string(),
                filename: backup_name,
                time: now,
            })
            .await?;

        let rclone_destination = format!("{}:{surreal_id}", &state.rclone_remote);
        let _ = rclone(&[
            "copy",
            &backup_destination,
            &rclone_destination,
            "--protondrive-replace-existing-draft=true",
            "--config",
            &state.rclone_conf_file,
        ])
        .await?;

        let output = rclone(&[
            "ls",
            &rclone_destination,
            "--config",
            &state.rclone_conf_file,
        ])
        .await?;

        let oldest_backup_time_to_keep = now - Duration::from_secs(state.max_backup_age);
        let file_name_parse_string = format!("{surreal_id}-%Y-%m-%d_%H-%M-%S.tar.zst");

        let remote_files_to_delete = output
            .lines()
            .filter_map(|line| {
                if !line.trim().is_empty() {
                    let (_size_in_bytes, file_name) = line.split_once(" ").unwrap();
                    match NaiveDateTime::parse_from_str(file_name, file_name_parse_string.as_str())
                    {
                        Ok(file_time) => {
                            if file_time.and_utc() < oldest_backup_time_to_keep {
                                return Some(format!(
                                    "{}:{surreal_id}/{file_name}",
                                    &state.rclone_remote
                                ));
                            }
                        }
                        Err(e) => {
                            tracing::error!("Cleanup error: {line}: {e}");
                        }
                    }
                }
                None
            })
            .collect::<Vec<String>>();

        for file in remote_files_to_delete {
            let _ = rclone(&["deletefile", &file, "--config", &state.rclone_conf_file]).await?;
        }

        fs::read_dir(&local_backup_dir)?.for_each(|entry| {
            if let Ok(entry) = entry {
                if entry.path().is_file() {
                    let file_name = entry.file_name().into_string().unwrap();
                    // let file_time =
                    //     NaiveDateTime::parse_from_str(&file_name, file_name_parse_string.as_str())
                    //         .unwrap()
                    //         .and_utc();
                    // TODO: sometimes uploads fail
                    // if file_time < oldest_backup_time_to_keep {
                    let source = format!("{local_backup_dir}/{}", &file_name);
                    fs::remove_file(source).unwrap();
                    // }
                }
            }
        });
    }
    Ok(())
}

#[cfg(feature = "backups")]
pub async fn backup_world(state: &AppState) -> Result<(), AppError> {
    backup_data_dir(
        &state.minecraft_modded_rcon_address,
        &state.minecraft_modded_rcon_password,
        state.minecraft_modded_data_dir.clone(),
        "minecraft_modded",
        &state.minecraft_modded_service_name,
        &state.minecraft_modded_connection,
        state,
    )
    .await?;
    backup_data_dir(
        &state.minecraft_geyser_rcon_address,
        &state.minecraft_geyser_rcon_password,
        state.minecraft_geyser_data_dir.clone(),
        "minecraft_geyser",
        &state.minecraft_geyser_service_name,
        &state.minecraft_geyser_connection,
        state,
    )
    .await?;
    Ok(())
}
