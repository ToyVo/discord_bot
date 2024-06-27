use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
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

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        tasks: Mutex::new(Vec::new()),
    });

    let app = Router::new()
        .route("/start_task", post(handle_request))
        .route("/check_status", get(check_status))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}