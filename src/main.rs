use axum::{
    extract::ConnectInfo,
    routing::{get, post},
    response::{Json, Html},
    Router,
    http::HeaderMap,
};
use serde_json::{json, Value};
use std::{
    env::var,
    net::SocketAddr,
};
use axum::http::StatusCode;
use serenity::interactions_endpoint::Verifier;
use crate::{
    commands::create_all_commands,
    utils::{get_random_emoji, install_global_commands},
};

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
    Pong = 1, // ACK a Ping
    ChannelMessageWithSource = 4, //respond to an interaction with a message
    DeferredChannelMessageWithSource = 5, //ACK an interaction and edit a response later, the user sees a loading state
    DeferredUpdateMessage = 6, // for components, ACK an interaction and edit the original message later; the user does not see a loading state
    UpdateMessage = 7, // for components, edit the message the component was attached to
    ApplicationCommandAutocompleteResult = 8, // respond to an autocomplete interaction with suggested choices
    Modal = 9, // respond to an interaction with a popup modal
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    install_global_commands(create_all_commands()).await.expect("Cannot update slash commands");
    let host = var("HOST").unwrap_or(String::from("0.0.0.0"));
    let port = var("PORT").unwrap_or(String::from("8080"));
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app().into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
    Ok(())
}

// Having a function that produces our app makes it easy to call it from tests
// without having to create an HTTP server.
fn app() -> Router {
    Router::new()
        .route("/", get(|| async { Html("<h1>Hello, World!</h1>") }))
        .route(
            "/json",
            post(|payload: Json<Value>| async move {
                Json(json!({ "data": payload.0 }))
            }),
        )
        .route(
            "/requires-connect-info",
            get(|ConnectInfo(addr): ConnectInfo<SocketAddr>| async move { format!("Hi {addr}") }),
        )
        .route(
            "/api/interactions",
            post(interactions),
        )
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
}

/// Interactions endpoint URL where Discord will send HTTP requests
async fn interactions(headers: HeaderMap, body: String) -> (StatusCode, Json<Value>) {
    // Parse request body and verifies incoming requests
    let verifier = Verifier::new(var("DISCORD_PUBLIC_KEY").expect("Could not get discord public key").as_str());
    let signature = headers.get("X-Signature-Ed25519").expect("Could not get signature from header").to_str().expect("could not convert signature header to string");
    let timestamp = headers.get("X-Signature-Timestamp").expect("could not get timestamp from header").to_str().expect("Could not convert timestamp header to string");
    if verifier.verify(signature, timestamp, body.as_ref()).is_err() {
        return (StatusCode::BAD_REQUEST, Json(json!({})));
    }

    let payload = Json::<Value>::from_bytes(body.as_bytes()).expect("Could not parse body");

    // Interaction type and data
    if let Some(request_type) = payload.get("type") {
        if let Some(request_type) = request_type.as_u64() {

            // Handle verification requests
            if request_type == DiscordInteractionType::Ping as u64 {
                return (StatusCode::OK, Json(json!({ "type": DiscordInteractionResponseType::Pong as u64})));
            }

            // Handle slash command requests
            // See https://discord.com/developers/docs/interactions/application-commands#slash-commands
            if request_type == DiscordInteractionType::ApplicationCommand as u64 {
                if let Some(data) = payload.get("data") {
                    if let Some(name) = data.get("name") {
                        if let Some(name) = name.as_str() {
                            // "test" command
                            if name == "mc" {
                                // Send a message into the channel where command was triggered from
                                return (StatusCode::OK, Json(json!({
                                    "type": DiscordInteractionResponseType::ChannelMessageWithSource as u64,
                                    "data": {
                                        // Fetches a random emoji to send from a helper function
                                        "content": format!("hello world {}", get_random_emoji()),
                                    },
                                })));
                            }
                        }
                    }
                }
            }
        }
    }

    (StatusCode::OK, Json(json!({})))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        extract::connect_info::MockConnectInfo,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    // for `collect`
    use serde_json::{json, Value};
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
    async fn json() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/json")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_vec(&json!([1, 2, 3, 4])).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body, json!({ "data": [1, 2, 3, 4] }));
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

    // Here we're calling `/requires-connect-info` which requires `ConnectInfo`
    //
    // That is normally set with `Router::into_make_service_with_connect_info` but we can't easily
    // use that during tests. The solution is instead to set the `MockConnectInfo` layer during
    // tests.
    #[tokio::test]
    async fn with_into_make_service_with_connect_info() {
        let mut app = app()
            .layer(MockConnectInfo(SocketAddr::from(([0, 0, 0, 0], 3000))))
            .into_service();

        let request = Request::builder()
            .uri("/requires-connect-info")
            .body(Body::empty())
            .unwrap();
        let response = app.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
