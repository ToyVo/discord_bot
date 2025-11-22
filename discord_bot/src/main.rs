#[cfg(feature = "server")]
use {
    axum::{
        Router,
        body::Body,
        extract::{Request, State},
        http::{HeaderValue, header},
    },
    dioxus::server::{DioxusRouterExt, FullstackState, ServeConfig},
    discord_bot::{
        discord,
        error::AppError,
        state::{AppState, set_global_state},
    },
    poise::serenity_prelude as serenity,
    std::{env::var, sync::Arc},
    tokio::sync::{Mutex, watch},
    tower_http::{
        LatencyUnit, ServiceBuilderExt,
        timeout::TimeoutLayer,
        trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    },
};

#[cfg(not(feature = "server"))]
fn main() {
    // The `launch` function is the main entry point for a dioxus app. It takes a component and renders it with the platform feature
    // you have enabled
    dioxus::launch(discord_bot::app::App);
}

#[cfg(feature = "server")]
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), discord_bot::error::AppError> {
    // On the server, we can use `dioxus::serve` and `.serve_dioxus_application` to serve our app with routing.
    // The `dioxus::server::router` function creates a new axum Router with the necessary routes to serve the Dioxus app.
    dioxus_logger::initialize_default();
    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
    //             format!(
    //                 "{}=debug,tower_http=debug,axum=trace",
    //                 env!("CARGO_CRATE_NAME")
    //             )
    //             .into()
    //         }),
    //     )
    //     .with(tracing_subscriber::fmt::layer().without_time())
    //     .init();

    // Build shared state
    let shared_state = Arc::new(Mutex::new(AppState {
        base_url: var("BASE_URL").unwrap_or_default(),
        discord_client_id: var("DISCORD_CLIENT_ID").unwrap_or_default(),
        discord_client_secret: var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
        discord_public_key: var("DISCORD_PUBLIC_KEY").unwrap_or_default(),
        discord_token: var("DISCORD_TOKEN").unwrap_or_default(),
        user_agent: format!(
            "DiscordBot ({}, {})",
            env!("CARGO_PKG_REPOSITORY"),
            env!("CARGO_PKG_VERSION")
        ),
        ..Default::default()
    }));

    // Set global state for Dioxus server functions
    set_global_state(shared_state.clone());

    // Shutdown signal channel we can use to shut down both tasks gracefully if desired.
    // We'll send a value when ctrl-c is received.
    let (shutdown_tx, shutdown_rx) = watch::channel::<()>(());

    // ------------- Interval -------------
    let shutdown_rx_for_interval = shutdown_rx.clone();
    let interval_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let mut shutdown_rx = shutdown_rx_for_interval.clone();
        // Start the interval with graceful shutdown
        tokio::select! {
            _ = async {
                loop {
                    interval.tick().await;
                    tracing::debug!("Tracking Tick");
                    // TODO: call game_manager to get the current game state
                }
            } => {
                tracing::error!("Interval task exited first");
            }
            _ = async {
                let _ = shutdown_rx.changed().await;
            } => {
                tracing::info!("Interval task received shutdown signal");
            }
        }
    });

    // ------------- Axum -------------
    let app_state_for_axum = shared_state.clone();

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

    // we want a deep integration of axum, dioxus, and state management, so we need to reimplement the dioxus axum wrapper, dioxus::server::router
    let mut router = Router::new()
        .layer(middleware)
        .with_state(app_state_for_axum)
        .serve_dioxus_application(ServeConfig::new(), discord_bot::app::App);

    // dioxus::server::base_path() is not public, so reimplement it, used in the dioxus::server::router function which we are reimplementing
    let base_path = dioxus_cli_config::base_path().map(|s| s.to_string());
    if let Some(base_path) = base_path {
        let base_path = base_path.trim_matches('/');

        // If there is a base path, nest the router under it and serve the root route manually
        // Nesting a route in axum only serves /base_path or /base_path/ not both
        router = Router::new().nest(&format!("/{base_path}/"), router).route(
            &format!("/{base_path}"),
            axum::routing::method_routing::get(
                |state: State<FullstackState>, mut request: Request<Body>| async move {
                    // The root of the base path always looks like the root from dioxus fullstack
                    *request.uri_mut() = "/".parse().unwrap();
                    FullstackState::render_handler(state, request).await
                },
            )
            .with_state(FullstackState::new(
                ServeConfig::new(),
                discord_bot::app::App,
            )),
        )
    }

    let address = dioxus_cli_config::fullstack_address_or_localhost();
    let listener = tokio::net::TcpListener::bind(address).await?;
    let mut shutdown_rx_for_axum = shutdown_rx.clone();
    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
        // Wait for the shutdown notification
        let _ = shutdown_rx_for_axum.changed().await;
    });

    // Spawn the Dioxus fullstack server as a background task
    let axum_handle = tokio::spawn(async move {
        tracing::trace!("Listening on {address}");
        if let Err(e) = server.await {
            tracing::error!("Dioxus server error: {}", e);
        }
    });

    // ------------- Poise -------------
    let shared_state_for_poise = shared_state.clone();
    let shutdown_rx_for_poise = shutdown_rx.clone();
    let poise_task = tokio::spawn(async move {
        let token = std::env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");
        let intents = serenity::GatewayIntents::non_privileged()
            | serenity::GatewayIntents::GUILD_MEMBERS
            | serenity::GatewayIntents::MESSAGE_CONTENT;

        let shared_state_for_setup = shared_state_for_poise.clone();
        let framework = poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![
                    discord::minecraft_geyser(),
                    discord::minecraft_modded(),
                    discord::terraria(),
                    discord::game_roles(),
                    discord::register_self_assignable_role(),
                    discord::deregister_self_assignable_role(),
                ],
                event_handler: |ctx, event, framework, data| {
                    Box::pin(discord::event_handler(ctx, event, framework, data))
                },
                ..Default::default()
            })
            .setup(move |ctx, _ready, framework| {
                Box::pin(async move {
                    tracing::info!("Logged in as {}", _ready.user.name);

                    // Register slash commands
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                    // set the activity of the bot
                    ctx.set_activity(Some(serenity::ActivityData::custom("Under Development")));

                    Ok(shared_state_for_setup)
                })
            })
            .build();

        tracing::info!("Starting Poise bot with shared state");

        // Create Serenity client with the framework
        let mut client = serenity::ClientBuilder::new(token, intents)
            .framework(framework)
            .await?;

        // Start the framework with graceful shutdown
        let mut shutdown_rx = shutdown_rx_for_poise.clone();
        tokio::select! {
            result = client.start() => {
                if let Err(e) = result {
                    tracing::error!("Serenity client error: {}", e);
                }
            }
            _ = async {
                let _ = shutdown_rx.changed().await;
            } => {
                tracing::info!("Poise bot received shutdown signal");
            }
        }

        tracing::info!("Poise bot exiting");
        Ok::<(), AppError>(())
    });

    // ------------- Shutdown Signal -------------
    // Wait for Ctrl+C or terminate signal and then trigger graceful shutdown
    shutdown_signal().await;
    tracing::info!("Shutdown signal received, shutting down...");
    let _ = shutdown_tx.send(());

    // Wait for all tasks to exit
    let _ = axum_handle.await;
    let _ = poise_task.await;
    let _ = interval_handle.await;

    tracing::info!("Shutdown complete");
    Ok(())
}

#[cfg(feature = "server")]
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
