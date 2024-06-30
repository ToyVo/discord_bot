use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use axum::extract::Path;
use reqwest::Method;
use tokio::sync::Mutex;

// Shared state to store task results
struct AppState {
    tasks: Mutex<Vec<String>>,
}

async fn handle_request(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Acknowledge the request immediately
    let task_id = uuid::Uuid::new_v4().to_string();
    let task_id_clone = task_id.clone();
    
    // Start background processing
    tokio::spawn(async move {
        // Simulate long-running task
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        
        // Store result
        let mut tasks = state.tasks.lock().await;
        tasks.push(format!("Task {} completed", task_id_clone));
    });

    // Return immediate response with task ID
    Json(json!({ "task_id": task_id, "message": "Task started" }))
}

async fn check_status(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tasks = state.tasks.lock().await;
    Json(json!({ "completed_tasks": tasks.len() }))
}

async fn send_slash_command() -> impl IntoResponse {
    let builder = reqwest::Client::new()
        .request(Method::POST, "http://localhost:8080/api/interactions")
        .header("Content-Type", "application/json; charset=UTF-8")
        .json(&json!({
            "type": 2,
            "token": "A_UNIQUE_TOKEN",
            "member": {
                "user": {
                    "id": "53908232506183680",
                    "username": "Mason",
                    "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432",
                    "discriminator": "1337",
                    "public_flags": 131141
                },
                "roles": ["539082325061836999"],
                "premium_since": null,
                "permissions": "2147483647",
                "pending": false,
                "nick": null,
                "mute": false,
                "joined_at": "2017-03-13T19:19:14.040000+00:00",
                "is_pending": false,
                "deaf": false
            },
            "id": "786008729715212338",
            "guild_id": "290926798626357999",
            "app_permissions": "442368",
            "guild_locale": "en-US",
            "locale": "en-US",
            "data": {
                "options": [{
                    "type": 3,
                    "name": "action",
                    "value": "Reboot"
                }],
                "type": 1,
                "name": "mc",
                "id": "771825006014889984"
            },
            "channel_id": "645027906669510667"
        }));
    let res = match builder.send().await {
        Ok(res) => res.text().await.unwrap_or("Error getting res text".to_string()),
        Err(err) => format!("{err:#?}").to_string(),
    };
    println!("{res}");
    res
}

async fn receive_discord_commands(Path(app_id): Path<String>) -> impl IntoResponse {
    println!("/applications/{app_id}/commands called");
    format!("/applications/{app_id}/commands called")
}

async fn receive_discord_webhook(Path(params): Path<(String, String)>) -> impl IntoResponse {
    let app_id = &params.0;
    let token = &params.1;
    println!("/webhooks/{app_id}/{token} called");
    format!("/webhooks/{app_id}/{token} called")
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        tasks: Mutex::new(Vec::new()),
    });

    let app = Router::new()
        .route("/start_task", post(handle_request))
        .route("/check_status", get(check_status))
        // endpoint to initiate test
        .route("/sample_slash_command", post(send_slash_command))
        // endpoint to receive test and mimic discord
        .route("/applications/:app_id/commands", post(receive_discord_commands))
        .route("/webhooks/:app_id/:token", post(receive_discord_webhook))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}