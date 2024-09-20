use axum_extra::extract::cookie::Key;
use serenity::all::{CommandOptionType, CommandType, CreateCommand, CreateCommandOption};
use std::sync::Arc;
use std::{env::var, net::SocketAddr, time::Duration};
use tokio::sync::RwLock;
use tokio::{net::TcpListener, signal};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::discord_utils::install_global_commands;
use crate::routes::{app, AppState, InnerState};

mod discord_utils;
mod handlers;
mod routes;
mod terraria;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "discord_bot=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let state = AppState(Arc::new(InnerState {
        key: Key::generate(),
        bot_token: var("DISCORD_BOT_TOKEN").unwrap_or_default(),
        client_id: var("DISCORD_CLIENT_ID").unwrap_or_default(),
        client_secret: var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
        public_key: var("DISCORD_PUBLIC_KEY").unwrap_or_default(),
        base_url: var("BASE_URL").unwrap_or_default(),
        user_agent: format!(
            "DiscordBot ({}, {})",
            env!("CARGO_PKG_REPOSITORY"),
            env!("CARGO_PKG_VERSION")
        ),
        minecraft_service_name: var("DISCORD_BOT_MINECRAFT_SERVICE_NAME")
            .unwrap_or(String::from("podman-minecraft.service")),
        terraria_service_name: var("DISCORD_BOT_TERRARIA_SERVICE_NAME")
            .unwrap_or(String::from("podman-terraria.service")),
        tshock_base_url: var("DISCORD_BOT_TSHOCK_REST_BASE_URL")
            .unwrap_or(String::from("http://localhost:7878")),
        tshock_token: var("TSHOCK_APPLICATION_TOKEN").unwrap_or_default(),
        terraria_players: RwLock::new(vec![]),
    }));

    let interval_state = state.clone();

    let commands = [
        CreateCommand::new("minecraft")
            .kind(CommandType::ChatInput)
            .description("Minecraft slash commands")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "available actions")
                    .required(true)
                    .add_string_choice("Reboot", "reboot"),
            ),
        CreateCommand::new("terraria")
            .kind(CommandType::ChatInput)
            .description("Terraria slash commands")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "available actions")
                    .required(true)
                    .add_string_choice("Broadcast", "broadcast")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::String, "message", "message")
                            .required(true),
                    ),
            ),
    ];

    if let Err(e) = install_global_commands(&commands, &state).await {
        tracing::error!("Failed to update slash commands\n{e:#?}");
    }

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tracing::debug!("Interval tick");
            interval.tick().await;
            if let Err(e) = terraria::track_players(&interval_state).await {
                tracing::error!("Failed to get status from terraria\n{e:#?}");
            }
        }
    });

    let host = var("HOST").unwrap_or_else(|_| String::from("0.0.0.0"));
    let port = var("PORT").unwrap_or_else(|_| String::from("8080"));
    let addr = format!("{}:{}", host, port);

    match TcpListener::bind(format!("{host}:{port}")).await {
        Ok(listener) => {
            tracing::info!("Listening on http://{addr}");
            if let Err(e) = axum::serve(
                listener,
                app()
                    .layer((
                        TraceLayer::new_for_http(),
                        TimeoutLayer::new(Duration::from_secs(10)),
                    ))
                    .with_state(state)
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal())
            .await
            {
                tracing::error!("Failed to start service\n{e:#?}");
            }
        }
        Err(e) => tracing::error!("Failed to bind listener\n{e:#?}"),
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
