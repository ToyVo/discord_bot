use std::{env::var, net::SocketAddr, time::Duration};

use axum::{
    http::{HeaderMap, StatusCode},
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use serenity::interactions_endpoint::Verifier;
use tokio::{net::TcpListener, process::Command, signal, time::sleep};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{commands::create_all_commands, utils::install_global_commands};

mod commands;
mod utils;

#[allow(dead_code)]
enum DiscordInteractionType {
    Ping = 1,
    ApplicationCommand = 2,
    MessageComponent = 3,
    ApplicationCommandAutocomplete = 4,
    ModalSubmit = 5,
}

#[allow(dead_code)]
enum DiscordInteractionResponseType {
    Pong = 1,                                 // ACK a Ping
    ChannelMessageWithSource = 4,             //respond to an interaction with a message
    DeferredChannelMessageWithSource = 5, //ACK an interaction and edit a response later, the user sees a loading state
    DeferredUpdateMessage = 6, // for components, ACK an interaction and edit the original message later; the user does not see a loading state
    UpdateMessage = 7,         // for components, edit the message the component was attached to
    ApplicationCommandAutocompleteResult = 8, // respond to an autocomplete interaction with suggested choices
    Modal = 9,                                // respond to an interaction with a popup modal
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "mc_discord_bot=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    if let Err(e) = install_global_commands(create_all_commands()).await {
        eprintln!("Failed to update slash commands\n{e:#?}");
    }
    let host = var("HOST").unwrap_or(String::from("0.0.0.0"));
    let port = var("PORT").unwrap_or(String::from("8080"));
    match TcpListener::bind(format!("{host}:{port}")).await {
        Ok(listener) => {
            match listener.local_addr() {
                Ok(addr) => println!("Listening on http://{addr}"),
                Err(e) => eprintln!("Failed to get addr off listener\n{e:#?}"),
            }
            if let Err(e) = axum::serve(
                listener,
                app()
                    .layer((
                        TraceLayer::new_for_http(),
                        TimeoutLayer::new(Duration::from_secs(10)),
                    ))
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal())
            .await
            {
                eprintln!("Failed to start service\n{e:#?}");
            }
        }
        Err(e) => eprintln!("Failed to bind listener\n{e:#?}"),
    }
    Ok(())
}

// Having a function that produces our app makes it easy to call it from tests
// without having to create an HTTP server.
fn app() -> Router {
    Router::new()
        .route("/", get(|| async { Html("<h1>Hello, World!</h1>") }))
        .route("/api/interactions", post(interactions))
        .route(
            "/verify-user",
            get(|| async { Html("<h1>Verify User</h1>") }),
        )
        .route(
            "/terms-of-service",
            get(|| async { Html("<h1>Terms of Service</h1>") }),
        )
        .route(
            "/privacy-policy",
            get(|| async { Html("<h1>Privacy Policy</h1>") }),
        )
        .route("/slow", get(|| sleep(Duration::from_secs(5))))
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

/// Interactions endpoint URL where Discord will send HTTP requests
async fn interactions(headers: HeaderMap, body: String) -> (StatusCode, Json<Value>) {
    println!("Request received: {body}");
    // Parse request body and verifies incoming requests
    let public_key = match var("DISCORD_PUBLIC_KEY") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Could not get discord public key\n{e:#?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({})));
        }
    };
    let verifier = Verifier::new(public_key.as_str());
    let signature = match headers.get("X-Signature-Ed25519") {
        Some(s) => match s.to_str() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Could not parse discord signature\n{e:#?}");
                return (StatusCode::BAD_REQUEST, Json(json!({})));
            }
        },
        None => {
            eprintln!("Could not get discord signature");
            return (StatusCode::BAD_REQUEST, Json(json!({})));
        }
    };
    let timestamp = match headers.get("X-Signature-Timestamp") {
        Some(s) => match s.to_str() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Could not parse discord timestamp\n{e:#?}");
                return (StatusCode::BAD_REQUEST, Json(json!({})));
            }
        },
        None => {
            eprintln!("Could not get discord timestamp");
            return (StatusCode::BAD_REQUEST, Json(json!({})));
        }
    };
    if verifier
        .verify(signature, timestamp, body.as_ref())
        .is_err()
    {
        eprintln!("Signature verification failed:\n\t{signature}\n\t{timestamp}");
        return (StatusCode::BAD_REQUEST, Json(json!({})));
    }

    let payload = match Json::<Value>::from_bytes(body.as_bytes()) {
        Ok(payload) => payload,
        Err(e) => {
            eprintln!("Could not parse body\n{e:#?}");
            return (StatusCode::BAD_REQUEST, Json(json!({})));
        }
    };

    let request_type = match payload.get("type") {
        Some(s) => s.as_u64(),
        None => {
            eprintln!("Could not get discord request type");
            None
        }
    };

    // Handle verification requests
    if request_type == Some(DiscordInteractionType::Ping as u64) {
        println!("Received discord ping request, Replying pong");
        return (
            StatusCode::OK,
            Json(json!({ "type": DiscordInteractionResponseType::Pong as u64})),
        );
    }

    // Handle slash command requests
    // See https://discord.com/developers/docs/interactions/application-commands#slash-commands
    if request_type == Some(DiscordInteractionType::ApplicationCommand as u64) {
        let command = match payload.get("data") {
            Some(s) => match s.get("name") {
                Some(n) => n.as_str(),
                None => {
                    eprintln!("Could not get discord slash command name");
                    None
                }
            },
            None => {
                eprintln!("Could not get discord slash command payload");
                None
            }
        };
        println!("Received discord slash command request, {command:#?}");
        if command == Some("mc") {
            // TODO: verify the argument and single source of truth for this and install_global_commands
            let content = match Command::new("systemctl").args(&["restart", "podman-minecraft.service"]).output().await {
                Ok(_) => String::from("Successfully restarted minecraft server, it might take a couple minutes to come up"),
                Err(e) => {
                    eprintln!("Could not restart minecraft server\n{e:#?}");
                    String::from("There was an issue restarting minecraft server")
                },
            };

            return (
                StatusCode::OK,
                Json(json!({
                    "type": DiscordInteractionResponseType::ChannelMessageWithSource as u64,
                    "data": {
                        "content": content,
                    },
                })),
            );
        }
    }

    (StatusCode::OK, Json(json!({})))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tokio::net::TcpListener;
    use tower::{Service, ServiceExt};

    use super::*;

    // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn hello_world() {
        let app = app();

        // `Router` implements `tower::Service<Request<Body>>` so we can
        // call it like any tower service, no need to run an HTTP server.
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"<h1>Hello, World!</h1>");
    }

    #[tokio::test]
    async fn not_found() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());
    }

    // You can also spawn a server and talk to it like any other HTTP server:
    #[tokio::test]
    async fn the_real_deal() {
        let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app()).await.unwrap();
        });

        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://{addr}"))
                    .header("Host", "localhost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"<h1>Hello, World!</h1>");
    }

    // You can use `ready()` and `call()` to avoid using `clone()`
    // in multiple request
    #[tokio::test]
    async fn multiple_request() {
        let mut app = app().into_service();

        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
