use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dioxus::prelude::*;
use std::collections::HashMap;
use tokio::process::Command;

use crate::routes::{html_app, AppState};

pub async fn log_viewer_endpoint(
    Query(query): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let unit = query
        .get("unit")
        .unwrap_or(&state.minecraft_modded_service_name);

    if unit.as_str() != state.minecraft_modded_service_name
        || unit.as_str() != state.minecraft_geyser_service_name
        || unit.as_str() != state.terraria_service_name
    {
        return (StatusCode::BAD_REQUEST, html_app(rsx! {"400"}, "400"));
    }

    let since = if let Some(since) = query.get("since") {
        since.as_str()
    } else {
        "1 hour ago"
    };
    let until = query.get("until");

    let mut journalctl_args = vec!["-u", unit, "-S", since];

    if let Some(until) = until {
        journalctl_args.push("-U");
        journalctl_args.push(until);
    }

    let output = Command::new("journalctl")
        .args(journalctl_args)
        .output()
        .await;

    match output {
        Ok(output) => {
            let logs = std::str::from_utf8(&output.stdout);
            match logs {
                Ok(logs) => (
                    StatusCode::OK,
                    html_app(
                        rsx! {
                            div {
                                {logs}
                            }
                        },
                        "Log Viewer",
                    ),
                ),
                Err(e) => {
                    tracing::error!("failed to parse journalctl: {e:#?}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        html_app(
                            rsx! {
                                "500"
                            },
                            "500",
                        ),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to run journalctl: {e:#?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                html_app(
                    rsx! {
                        "500"
                    },
                    "500",
                ),
            )
        }
    }
}
