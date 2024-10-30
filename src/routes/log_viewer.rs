use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dioxus::prelude::*;
use tokio::time::{Instant,Duration};
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

    let valid_services = [
        state.minecraft_modded_service_name.clone(),
        state.minecraft_geyser_service_name.clone(),
        state.terraria_service_name.clone(),
        String::from("discord_bot.service"),
    ];

    if !valid_services.contains(unit) {
        return (StatusCode::BAD_REQUEST, html_app(rsx! {"400"}, "400"));
    }

    let now = Instant::now();

    let since = if let Some(since) = query.get("since") {
        since.as_str()
    } else {
        let ts = now - Duration::from_secs(3600);
        "1 hour ago"
    };

    let mut journalctl_args = vec!["-u", unit, "-S", since];

    if let Some(until) = query.get("until") {
        journalctl_args.push("-U");
        journalctl_args.push(until);
    }

    tracing::debug!("Fetching logs: {journalctl_args:#?}");

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
                                label {
                                    r#for: "unit-select",
                                    "Unit"
                                }
                                select {
                                    id: "unit-select",
                                    for service in valid_services {
                                        option {
                                            value: service.clone(),
                                            {service.clone()}
                                        }
                                    }
                                }
                                label {
                                    r#for: "since-input",
                                    "Since"
                                }
                                input {
                                    id: "since-input",
                                    r#type: "datetime-local",
                                    value: ""
                                }
                                label {
                                    r#for: "until-input",
                                    "Until"
                                }
                                input {
                                    id: "until-input",
                                    r#type: "datetime-local",
                                    value: ""
                                }
                            }
                            pre {
                                margin: 0,
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
