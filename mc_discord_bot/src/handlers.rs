use std::env::var;
use axum::Json;
use reqwest::Method;
use serde_json::{json, Value};
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage};
use serenity::json;
use tokio::process::Command;
use crate::discord_utils::discord_request;

pub async fn handle_slash_command(payload: Json<Value>) -> Value {
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
        let token = match payload.get("token") {
            Some(s) => match s.as_str() {
                Some(s) => s.to_string(),
                None => "unavailable".to_string(),
            },
            None => {
                eprintln!("Could not get discord interaction token");
                "unavailable".to_string()
            }
        };
        let app_id = var("DISCORD_CLIENT_ID").unwrap_or_default();

        // TODO: verify the argument and single source of truth for this and install_global_commands
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
            discord_request(
                format!("webhooks/{app_id}/{token}"),
                Method::POST,
                Some(&CreateInteractionResponseFollowup::new().content(content)),
            )
                .await
                .unwrap();
        });

        return json::to_value(CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("Successfully requested restart of minecraft server, it might take a couple minutes to come up"))).unwrap();
    }
    json!({})
}
