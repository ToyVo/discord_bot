use crate::error::AppError;
use crate::routes::{html_app, AppState};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::prelude::*;
use dioxus::prelude::*;
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use tokio::process::Command;
use tokio::time::Duration;

async fn get_logs(args: &[&str]) -> Result<String, AppError> {
    #[cfg(target_os = "linux")]
    let output = Command::new("journalctl").args(args).output().await?;
    #[cfg(target_os = "linux")]
    let logs = std::str::from_utf8(&output.stdout)?;
    #[cfg(target_os = "linux")]
    let logs = logs.to_string();
    #[cfg(not(target_os = "linux"))]
    let logs = format!("No logs available on this platform. {args:#?}");
    Ok(logs)
}

pub async fn log_viewer_endpoint(
    Query(query): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let unit = query
        .get("unit")
        .unwrap_or(&state.minecraft_modded_service_name);

    tracing::debug!("{query:#?}");

    let valid_services = [
        state.minecraft_modded_service_name.clone(),
        state.minecraft_geyser_service_name.clone(),
        state.terraria_service_name.clone(),
        String::from("discord_bot.service"),
    ];

    if !valid_services.contains(unit) {
        return (StatusCode::BAD_REQUEST, html_app(rsx! {"400"}, "400"));
    }

    let now = Utc::now();

    let since = if let Some(since) = query.get("since") {
        format!("{since}z")
            .parse::<DateTime<Utc>>()
            .unwrap_or(now - Duration::from_secs(3600))
    } else {
        now - Duration::from_secs(3600)
    };

    let until = if let Some(until) = query.get("until") {
        format!("{until}z").parse::<DateTime<Utc>>().unwrap_or(now)
    } else {
        now
    };

    let since_string = since.to_rfc3339();
    let until_string = since.to_rfc3339();
    let journalctl_args = vec![
        "--utc",
        "-u",
        unit,
        "-S",
        since_string.as_str(),
        "-U",
        until_string.as_str(),
    ];

    tracing::debug!("Fetching logs: {journalctl_args:#?}");

    match get_logs(&journalctl_args).await {
        Ok(logs) => (
            StatusCode::OK,
            html_app(
                rsx! {
                    form {
                        label {
                            r#for: "unit-select",
                            "Unit"
                        }
                        select {
                            id: "unit-select",
                            name: "unit",
                            for service in valid_services {
                                option {
                                    value: service.clone(),
                                    selected: service == unit.as_str(),
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
                            name: "since",
                            r#type: "datetime-local",
                            value: since.format("%Y-%m-%dT%H:%M:%S").to_string(),
                        }
                        label {
                            r#for: "until-input",
                            "Until"
                        }
                        input {
                            id: "until-input",
                            name: "until",
                            r#type: "datetime-local",
                            value: until.format("%Y-%m-%dT%H:%M:%S").to_string(),
                        }
                        small {
                            "Note: time shown in UTC"
                        }
                        button {
                            r#type: "submit",
                            "Fetch Logs"
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
            tracing::error!("failed to get logs: {e:#?}");
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
