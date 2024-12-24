use discord_bot::app::App;

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use axum::http::{header, HeaderValue};
    use axum_extra::extract::cookie::Key;
    use dioxus::prelude::{DioxusRouterExt, LaunchBuilder};
    use discord_bot::server::{
        discord, minecraft, shutdown_signal, terraria, AppState, InnerState,
    };
    use serenity::all::{CommandOptionType, CommandType, CreateCommand, CreateCommandOption};
    use std::env::var;
    use tower_http::{
        timeout::TimeoutLayer,
        trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
        LatencyUnit, ServiceBuilderExt,
    };
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "discord_bot=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let surrealdb_path =
        var("SURREALDB_PATH").unwrap_or(String::from("/var/lib/discord_bot/surrealdb"));
    let db = surrealdb::Surreal::new::<surrealdb::engine::local::RocksDb>(surrealdb_path)
        .await
        .unwrap();
    // Select a specific namespace / database
    db.use_ns(env!("CARGO_PKG_NAME"))
        .use_db(env!("CARGO_PKG_NAME"))
        .await
        .unwrap();

    let state = AppState(std::sync::Arc::new(InnerState {
        base_url: var("BASE_URL").unwrap_or_default(),
        client_id: var("DISCORD_CLIENT_ID").unwrap_or_default(),
        client_secret: var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
        cloud_ssh_host: var("CLOUD_SSH_HOST").ok(),
        db,
        discord_bot_spam_channel_id: var("DISCORD_BOT_SPAM_CHANNEL_ID").unwrap_or_default(),
        discord_minecraft_geyser_channel_id: var("DISCORD_MINECRAFT_GEYSER_CHANNEL_ID")
            .unwrap_or_default(),
        discord_minecraft_modded_channel_id: var("DISCORD_MINECRAFT_CHANNEL_ID")
            .unwrap_or_default(),
        discord_terraria_channel_id: var("DISCORD_TERRARIA_CHANNEL_ID").unwrap_or_default(),
        discord_token: var("DISCORD_TOKEN").unwrap_or_default(),
        forge_api_key: var("FORGE_API_KEY").unwrap_or_default(),
        key: Key::generate(),
        minecraft_geyser_rcon_address: var("MINECRAFT_GEYSER_RCON_ADDRESS")
            .unwrap_or(String::from("localhost:25576")),
        minecraft_geyser_rcon_password: var("RCON_PASSWORD").unwrap_or_default(),
        minecraft_modded_rcon_address: var("MINECRAFT_MODDED_RCON_ADDRESS")
            .unwrap_or(String::from("localhost:25575")),
        minecraft_modded_rcon_password: var("RCON_PASSWORD").unwrap_or_default(),
        public_key: var("DISCORD_PUBLIC_KEY").unwrap_or_default(),
        tshock_base_url: var("TSHOCK_REST_BASE_URL")
            .unwrap_or(String::from("http://localhost:7878")),
        tshock_token: var("TSHOCK_APPLICATION_TOKEN").unwrap_or_default(),
        user_agent: format!(
            "DiscordBot ({}, {})",
            env!("CARGO_PKG_REPOSITORY"),
            env!("CARGO_PKG_VERSION")
        ),
    }));

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
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::String, "message", "message")
                            .required(true),
                    ),
            ),
    ];

    if !state.discord_token.is_empty() {
        if let Err(e) = discord::install_global_commands(&commands, &state).await {
            tracing::error!("Failed to update slash commands: {e}");
        }
    }

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            tracing::debug!("Tracking Tick");
            if let Err(e) = terraria::track_players(&interval_state).await {
                tracing::error!("Failed to get players from terraria: {e}");
            }
            if let Err(e) = minecraft::track_players(&interval_state).await {
                tracing::error!("Failed to get players from minecraft: {e}");
            }
        }
    });

    let sensitive_headers: std::sync::Arc<[_]> = vec![header::AUTHORIZATION, header::COOKIE].into();
    // Build our middleware stack
    let middleware = tower::ServiceBuilder::new()
        // Mark the `Authorization` and `Cookie` headers as sensitive so it doesn't show in logs
        .sensitive_request_headers(sensitive_headers.clone())
        // Add high level tracing/logging to all requests
        .layer(
            TraceLayer::new_for_http()
                .on_body_chunk(|chunk: &axum::body::Bytes, latency: std::time::Duration, _: &tracing::Span| {
                    tracing::trace!(size_bytes = chunk.len(), latency = ?latency, "sending body chunk")
                })
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(DefaultOnResponse::new().include_headers(true).latency_unit(LatencyUnit::Micros)),
        )
        .sensitive_response_headers(sensitive_headers)
        // Set a timeout
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(10)))
        // Compress responses
        .compression()
        // Set a `Content-Type` if there isn't one already.
        .insert_response_header_if_not_present(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );

    let router = axum::Router::new()
        .route(
            "/api/discord/interactions",
            axum::routing::post(discord::interactions),
        )
        .route(
            "/api/discord/verify-user",
            axum::routing::get(discord::verify_user),
        )
        .route(
            "/api/discord/oauth-callback",
            axum::routing::get(discord::oauth_callback),
        )
        .serve_dioxus_application(LaunchBuilder::new().with_context(state), App)
        .layer(middleware)
        .with_state(state)
        .into_make_service();

    let address = dioxus_cli_config::fullstack_address_or_localhost();
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
