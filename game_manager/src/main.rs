#[cfg(feature = "server")]
use {
    axum::{
        Json, Router,
        body::Body,
        extract::{FromRequestParts, Request, State},
        http::{HeaderValue, header, request::Parts},
        response::{IntoResponse, Response},
        routing::{get, post},
    },
    axum_extra::headers::{Authorization, authorization::Bearer},
    dioxus::server::{DioxusRouterExt, FullstackState, ServeConfig},
    jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode},
    rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding},
    serde_json::json,
    std::{
        env::var,
        sync::{Arc, LazyLock},
        time::{SystemTime, UNIX_EPOCH},
    },
    tokio::sync::Mutex,
    tower_http::{
        LatencyUnit, ServiceBuilderExt,
        timeout::TimeoutLayer,
        trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    },
};
use {
    dioxus::prelude::*,
    serde::{Deserialize, Serialize},
    serde_json::Value,
};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
// Global state - this will be set by the main function
#[cfg(feature = "server")]
pub static GLOBAL_STATE: std::sync::OnceLock<Arc<Mutex<AppState>>> = std::sync::OnceLock::new();

#[cfg(feature = "server")]
pub static KEYS: LazyLock<(EncodingKey, DecodingKey)> = LazyLock::new(|| {
    let (private_key, encoding) = if let Ok(path) = var("JWT_PRIVATE_KEY") {
        let private_key_string = std::fs::read_to_string(path).unwrap();
        let private_key = rsa::RsaPrivateKey::from_pkcs1_pem(&private_key_string).unwrap();
        let encoding = EncodingKey::from_rsa_pem(private_key_string.as_bytes()).unwrap();
        (private_key, encoding)
    } else {
        let mut rng = rand::thread_rng();
        let private_key = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let private_key_string = private_key.to_pkcs1_pem(LineEnding::LF).unwrap();
        warn!("Using generated RSA key for JWT");
        let encoding = EncodingKey::from_rsa_pem(private_key_string.as_bytes()).unwrap();
        (private_key, encoding)
    };
    let public_key = private_key.to_public_key();
    let decoding =
        DecodingKey::from_rsa_pem(public_key.to_pkcs1_pem(LineEnding::LF).unwrap().as_bytes())
            .unwrap();
    (encoding, decoding)
});

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct AppState {
    pub minecraft_geyser_address: String,
    pub minecraft_modded_address: String,
    pub terraria_address: String,
    pub tshock_base_url: String,
    pub tshock_token: String,
}

#[cfg(feature = "server")]
pub fn set_global_state(state: Arc<Mutex<AppState>>) {
    let _ = GLOBAL_STATE.set(state);
}

#[cfg(not(feature = "server"))]
fn main() {
    // The `launch` function is the main entry point for a dioxus app. It takes a component and renders it with the platform feature
    // you have enabled
    dioxus::launch(App);
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        minecraft_geyser_address: "".into(),
        minecraft_modded_address: "".into(),
        terraria_address: "".into(),
        tshock_base_url: "".into(),
        tshock_token: "".into(),
    }));

    // Set global state for Dioxus server functions
    set_global_state(shared_state.clone());

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
        .route("/api/logs", get(logs))
        .route("/api/authorize", post(authorize))
        .layer(middleware)
        .with_state(app_state_for_axum)
        .serve_dioxus_application(ServeConfig::new(), App);

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
            .with_state(FullstackState::new(ServeConfig::new(), App)),
        )
    }

    let address = dioxus_cli_config::fullstack_address_or_localhost();
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::trace!("Listening on {address}");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
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

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        h1 { "game manager" }
    }
}

#[get("/api/minecraft/status")]
pub async fn minecraft_status() -> Result<Value, ServerFnError> {
    Ok(json!({
        "code": 200,
        "status": "ok"
    }))
}

#[get("/api/minecraft/players")]
pub async fn minecraft_players() -> Result<Value, ServerFnError> {
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
pub async fn terraria_status() -> Result<Value, ServerFnError> {
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
pub async fn terraria_players() -> Result<Value, ServerFnError> {
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

#[cfg(feature = "server")]
pub async fn logs(claims: Claims) -> Result<String, AuthError> {
    Ok(format!("Logs for user, {:?}", claims))
}

#[cfg(feature = "server")]
pub async fn authorize(Json(payload): Json<AuthPayload>) -> Result<Json<AuthBody>, AuthError> {
    // Check if the user sent the credentials
    if payload.client_id.is_empty() || payload.client_secret.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    // Here you can check the user credentials from a database
    if payload.client_id != "foo" || payload.client_secret != "bar" {
        return Err(AuthError::WrongCredentials);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = Claims {
        sub: "b@b.com".to_owned(),
        aud: "ACME".to_owned(),
        iss: "ACME".to_owned(),
        // Mandatory expiry time as UTC timestamp
        exp: now + 3600, // 1 hour from now
        iat: now,
        nbf: now,
    };
    // Create the authorization token
    let token =
        encode(&Header::new(Algorithm::RS256), &claims, &KEYS.0).map_err(|_| AuthError::TokenCreation)?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

#[derive(Debug)]
#[cfg(feature = "server")]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
}

#[cfg(feature = "server")]
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };
        let body = Json(json!({
            "status": status.as_u16(),
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

#[derive(Debug, Serialize)]
#[cfg(feature = "server")]
pub struct AuthBody {
    pub access_token: String,
    pub token_type: String,
}

#[cfg(feature = "server")]
impl AuthBody {
    pub fn new(access_token: String) -> Self {
        AuthBody {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "server")]
pub struct AuthPayload {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "server")]
pub struct Claims {
    aud: String, // Optional. Audience
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    nbf: usize, // Optional. Not Before (as UTC timestamp)
    sub: String, // Optional. Subject (whom token refers to)
}

#[cfg(feature = "server")]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header

        use axum_extra::headers::HeaderMapExt;
        let bearer = parts
            .headers
            .typed_get::<Authorization<Bearer>>()
            .ok_or(AuthError::InvalidToken)?;
        // Decode the user data
        let token_data = decode::<Claims>(bearer.token(), &KEYS.1, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}
