use axum::{
    body::Bytes,
    http::{header, HeaderValue},
};
use axum_extra::extract::cookie::Key;
use serenity::all::{CommandOptionType, CommandType, CreateCommand, CreateCommandOption};
use std::{env::var, net::SocketAddr, sync::Arc, time::Duration};
#[cfg(feature = "db")]
use surrealdb::engine::remote::ws::Ws;
#[cfg(feature = "db")]
use surrealdb::opt::auth::Root;
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit, ServiceBuilderExt,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use discord_bot::discord_utils::install_global_commands;
use discord_bot::routes::{app, AppState, InnerState};
#[cfg(feature = "db")]
use discord_bot::DB;
#[cfg(feature = "watchers")]
use discord_bot::{minecraft, terraria};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "discord_bot=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    #[cfg(feature = "db")]
    let surreal_bind = var("SURREAL_BIND").unwrap_or(String::from("127.0.0.1:8000"));
    #[cfg(feature = "db")]
    let surreal_pass = var("SURREAL_PASS").unwrap_or_default();
    #[cfg(feature = "db")]
    DB.connect::<Ws>(surreal_bind).await?;

    #[cfg(feature = "db")]
    DB.signin(Root {
        username: "root",
        password: surreal_pass.as_str(),
    })
    .await?;

    // Select a specific namespace / database
    #[cfg(feature = "db")]
    DB.use_ns(env!("CARGO_PKG_NAME"))
        .use_db(env!("CARGO_PKG_NAME"))
        .await?;

    let state = AppState(Arc::new(InnerState {
        base_url: var("BASE_URL").unwrap_or_default(),
        client_id: var("DISCORD_CLIENT_ID").unwrap_or_default(),
        client_secret: var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
        discord_bot_spam_channel_id: var("DISCORD_BOT_SPAM_CHANNEL_ID").unwrap_or_default(),
        discord_minecraft_geyser_channel_id: var("DISCORD_MINECRAFT_GEYSER_CHANNEL_ID")
            .unwrap_or_default(),
        discord_minecraft_modded_channel_id: var("DISCORD_MINECRAFT_CHANNEL_ID")
            .unwrap_or_default(),
        discord_terraria_channel_id: var("DISCORD_TERRARIA_CHANNEL_ID").unwrap_or_default(),
        discord_token: var("DISCORD_TOKEN").unwrap_or_default(),
        forge_api_key: var("FORGE_API_KEY").unwrap_or_default(),
        key: Key::generate(),
        #[cfg(feature = "watchers")]
        minecraft_geyser_connection: Default::default(),
        minecraft_geyser_data_dir: var("MINECRAFT_GEYSER_DATA_DIR")
            .unwrap_or(String::from("/minecraft-geyser-data")),
        minecraft_geyser_rcon_address: var("MINECRAFT_RCON_ADDRESS")
            .unwrap_or(String::from("localhost:25576")),
        minecraft_geyser_rcon_password: var("RCON_PASSWORD").unwrap_or_default(),
        minecraft_geyser_service_name: var("MINECRAFT_GEYSER_SERVICE_NAME")
            .unwrap_or(String::from("arion-minecraft-geyser.service")),
        #[cfg(feature = "watchers")]
        minecraft_modded_connection: Default::default(),
        minecraft_modded_data_dir: var("MINECRAFT_MODDED_DATA_DIR")
            .unwrap_or(String::from("/minecraft-modded-data")),
        minecraft_modded_rcon_address: var("MINECRAFT_RCON_ADDRESS")
            .unwrap_or(String::from("localhost:25575")),
        minecraft_modded_rcon_password: var("RCON_PASSWORD").unwrap_or_default(),
        minecraft_modded_service_name: var("MINECRAFT_MODDED_SERVICE_NAME")
            .unwrap_or(String::from("arion-minecraft-modded.service")),
        public_key: var("DISCORD_PUBLIC_KEY").unwrap_or_default(),
        #[cfg(feature = "backups")]
        rclone_conf_file: var("RCLONE_CONF_FILE").unwrap_or_default(),
        #[cfg(feature = "backups")]
        rclone_remote: var("RCLONE_REMOTE").unwrap_or(String::from("protondrive")),
        terraria_service_name: var("TERRARIA_SERVICE_NAME")
            .unwrap_or(String::from("arion-terraria.service")),
        tshock_base_url: var("TSHOCK_REST_BASE_URL")
            .unwrap_or(String::from("http://localhost:7878")),
        tshock_token: var("TSHOCK_APPLICATION_TOKEN").unwrap_or_default(),
        user_agent: format!(
            "DiscordBot ({}, {})",
            env!("CARGO_PKG_REPOSITORY"),
            env!("CARGO_PKG_VERSION")
        ),
    }));

    #[cfg(feature = "watchers")]
    let interval_state = state.clone();

    let commands = [
        CreateCommand::new("minecraft-geyser")
            .kind(CommandType::ChatInput)
            .description("Minecraft slash commands")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "available actions")
                    .required(true)
                    .add_string_choice("Stop", "stop")
                    .add_string_choice("Restart", "restart"),
            ),
        CreateCommand::new("minecraft-modded")
            .kind(CommandType::ChatInput)
            .description("Minecraft slash commands")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "available actions")
                    .required(true)
                    .add_string_choice("Stop", "stop")
                    .add_string_choice("Restart", "restart"),
            ),
        CreateCommand::new("terraria")
            .kind(CommandType::ChatInput)
            .description("Terraria slash commands")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "action", "available actions")
                    .required(true)
                    .add_string_choice("Stop", "stop")
                    .add_string_choice("Restart", "restart")
                    .add_string_choice("Broadcast", "broadcast")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::String, "message", "message")
                            .required(true),
                    ),
            ),
    ];

    if let Err(e) = install_global_commands(&commands, &state).await {
        tracing::error!("Failed to update slash commands: {e}");
    }

    #[cfg(feature = "watchers")]
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(e) = terraria::track_players(&interval_state).await {
                tracing::error!("Failed to get status from terraria: {e}");
            }
            if let Err(e) = minecraft::track_players(&interval_state).await {
                tracing::error!("Failed to get status from minecraft: {e}");
            }
            #[cfg(feature = "backups")]
            if let Err(e) = minecraft::backup_world(&interval_state).await {
                tracing::error!("Failed to backup minecraft: {e}");
            }
        }
    });

    let host = var("HOST").unwrap_or_else(|_| String::from("0.0.0.0"));
    let port = var("PORT").unwrap_or_else(|_| String::from("8080"));
    let addr = format!("{}:{}", host, port);

    match TcpListener::bind(format!("{host}:{port}")).await {
        Ok(listener) => {
            tracing::info!("Listening on http://{addr}");
            let sensitive_headers: Arc<[_]> = vec![header::AUTHORIZATION, header::COOKIE].into();
            // Build our middleware stack
            let middleware = ServiceBuilder::new()
                // Mark the `Authorization` and `Cookie` headers as sensitive so it doesn't show in logs
                .sensitive_request_headers(sensitive_headers.clone())
                // Add high level tracing/logging to all requests
                .layer(
                    TraceLayer::new_for_http()
                        .on_body_chunk(|chunk: &Bytes, latency: Duration, _: &tracing::Span| {
                            tracing::trace!(size_bytes = chunk.len(), latency = ?latency, "sending body chunk")
                        })
                        .make_span_with(DefaultMakeSpan::new().include_headers(true))
                        .on_response(DefaultOnResponse::new().include_headers(true).latency_unit(LatencyUnit::Micros)),
                )
                .sensitive_response_headers(sensitive_headers)
                // Set a timeout
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
                // Compress responses
                .compression()
                // Set a `Content-Type` if there isn't one already.
                .insert_response_header_if_not_present(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/octet-stream"),
                );
            if let Err(e) = axum::serve(
                listener,
                app()
                    .layer(middleware)
                    .with_state(state)
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal())
            .await
            {
                tracing::error!("Failed to start service: {e}");
            }
        }
        Err(e) => tracing::error!("Failed to bind listener: {e}"),
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
