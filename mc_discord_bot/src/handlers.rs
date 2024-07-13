use axum::Json;
use reqwest::Method;
use serde_json::{json, Value};
use serenity::builder::{
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};
use serenity::json;
use tokio::process::Command;

use lib::AppError;

use crate::discord_utils::discord_request;
use crate::routes::AppState;

pub async fn handle_slash_command(payload: Json<Value>, state: AppState) -> Result<Value, AppError> {
    let command = payload
        .get("data")
        .and_then(|s| s.get("name"))
        .and_then(|s| s.as_str())
        .unwrap_or_default();
    println!("Received discord slash command request, {command:#?}");
    if command == "mc" {
        let token = payload
            .get("token")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .to_string();
        let options = payload
            .get("data")
            .and_then(|s| s.get("options"))
            .and_then(|s| s.as_array());

        if let Some(options) = options {
            for option in options {
                let name = option
                    .get("name")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();
                let value = option
                    .get("value")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();
                if name == "action" && value == "reboot" {
                    // TODO: single source of truth for this and install_global_commands
                    tokio::spawn(async move {
                        let content = match Command::new("systemctl").args(["restart", state.service_name.as_str()]).output().await {
                            Ok(_) => {
                                String::from("Successfully restarted minecraft server, it might take a couple minutes to come up")
                            }
                            Err(e) => {
                                eprintln!("Could not restart minecraft server\n{e:#?}");
                                String::from("There was an issue restarting minecraft server")
                            }
                        };
                        if let Err(e) = discord_request(
                            format!("webhooks/{}/{token}", state.client_id),
                            Method::POST,
                            Some(&CreateInteractionResponseFollowup::new().content(content)),
                            &state,
                        )
                        .await
                        {
                            eprintln!("Error submitting followup {e:#?}")
                        }
                    });
                    return Ok(json::to_value(CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Successfully requested restart of minecraft server"),
                    ))?);
                }
            }
        }
    }
    Ok(json!({}))
}
