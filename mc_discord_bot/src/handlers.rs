use std::env::var;
use axum::Json;
use reqwest::Method;
use serde_json::{json, Value};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage};
use serenity::json;
use tokio::process::Command;
use lib::AppError;
use crate::discord_utils::discord_request;

pub async fn handle_slash_command(payload: Json<Value>) -> Result<Value, AppError> {
    let command = payload
        .get("data")
        .and_then(|s| s.get("name"))
        .and_then(|s| s.as_str()).unwrap_or_default();
    println!("Received discord slash command request, {command:#?}");
    if command == "mc" {
        let token = payload
            .get("token")
            .and_then(|s| s.as_str()).unwrap_or_default().to_string();
        let options = payload
            .get("data")
            .and_then(|s| s.get("options"))
            .and_then(|s| s.as_array());
        let app_id = var("DISCORD_CLIENT_ID").unwrap_or_default();
        
        if let Some(options) = options {
            for option in options {
                let name = option.get("name").and_then(|s| s.as_str()).unwrap_or_default();
                let value = option.get("value").and_then(|s| s.as_str()).unwrap_or_default();
                if name == "action" && value == "Reboot" {
                    // TODO: single source of truth for this and install_global_commands
                    tokio::spawn(async move {
                        let content = match Command::new("systemctl").args(&["restart", "podman-minecraft.service"]).output().await {
                            Ok(_) => {
                                String::from("Successfully restarted minecraft server, it might take a couple minutes to come up")
                            }
                            Err(e) => {
                                eprintln!("Could not restart minecraft server\n{e:#?}");
                                String::from("There was an issue restarting minecraft server")
                            }
                        };
                        if let Err(e) = discord_request(
                            format!("webhooks/{app_id}/{token}"),
                            Method::POST,
                            Some(&CreateInteractionResponseFollowup::new().content(content)),
                        ).await {
                            eprintln!("Error submitting followup {e:#?}")
                        }
                    });
                    return Ok(json::to_value(CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("Successfully requested restart of minecraft server")))?);
                }
            }
        }
    }
    Ok(json!({}))
}
