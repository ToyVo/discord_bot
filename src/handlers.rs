use axum::Json;
use reqwest::Method;
use serde_json::{json, Value};
use serenity::builder::{
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};
use serenity::json;
use tokio::process::Command;

use crate::discord_utils::discord_request;
use crate::error::AppError;
use crate::routes::AppState;
use crate::terraria;

pub async fn handle_slash_command(
    payload: Json<Value>,
    state: AppState,
) -> Result<Value, AppError> {
    tracing::info!("Received discord slash command request, {payload:#?}");
    let command = payload
        .get("data")
        .and_then(|s| s.get("name"))
        .and_then(|s| s.as_str())
        .unwrap_or_default();
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
                // TODO: single source of truth for this and install_global_commands
                match (name, value) {
                    ("action", "reboot") => {
                        tokio::spawn(async move {
                            let content = match Command::new("systemctl").args(["restart", state.minecraft_service_name.as_str()]).output().await {
                                Ok(_) => {
                                    String::from("Successfully restarted minecraft server, it might take a couple minutes to come up")
                                }
                                Err(e) => {
                                    tracing::error!("Could not restart minecraft server\n{e:#?}");
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
                                tracing::error!("Error submitting followup {e:#?}")
                            }
                        });
                        return Ok(json::to_value(CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("Successfully requested restart of minecraft server"),
                        ))?);
                    }
                    ("action", "broadcast") => {
                        tokio::spawn(async move {
                            let content = match terraria::broadcast(&state, "").await {
                                Ok(_) => String::from(
                                    "Successfully broadcast message to terraria server",
                                ),
                                Err(e) => {
                                    tracing::error!(
                                        "Could not send message to terraria server\n{e:#?}"
                                    );
                                    String::from(
                                        "There was an issue sending message to terraria server",
                                    )
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
                                tracing::error!("Error submitting followup {e:#?}")
                            }
                        });
                        return Ok(json::to_value(CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(
                                "Successfully requested broadcast of message to terraria server",
                            ),
                        ))?);
                    }
                    (_, _) => {}
                }
            }
        }
    }
    Ok(json!({}))
}
