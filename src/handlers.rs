use reqwest::Method;
use serde_json::{json, Value};
use serenity::all::{CommandDataOptionValue, CommandInteraction};
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
    payload: CommandInteraction,
    state: AppState,
) -> Result<Value, AppError> {
    tracing::info!("Received discord slash command request, {:#?}", &payload);
    for option in payload.data.options {
        // TODO: single source of truth for this and install_global_commands
        match (
            payload.data.name.as_str(),
            option.name.as_str(),
            option.value,
        ) {
            (
                "minecraft-geyser" | "minecraft-modded" | "terraria",
                "action",
                CommandDataOptionValue::String(s),
            ) if s == "restart" || s == "stop" => {
                let server = payload.data.name.clone();
                let action = s.clone();
                tokio::spawn(async move {
                    let server = payload.data.name;
                    let action = s.as_str();
                    let service_name = format!("arion-{server}.service");
                    let content = match Command::new("systemctl")
                        .args([action, service_name.as_str()])
                        .output()
                        .await
                    {
                        Ok(_) => {
                            format!("Successfully {action}ed {server} server")
                        }
                        Err(e) => {
                            tracing::error!("Could not {action} {server} server\n{e:#?}");
                            format!("There was an issue {action}ing {server} server")
                        }
                    };
                    if let Err(e) = discord_request(
                        format!("webhooks/{}/{}", state.client_id, payload.token),
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
                    CreateInteractionResponseMessage::new().content(format!(
                        "Successfully requested {action} of {server} server"
                    )),
                ))?);
            }
            ("terraria", "action", CommandDataOptionValue::String(s)) if s == "broadcast" => {
                tokio::spawn(async move {
                    let content = match terraria::broadcast(&state, "").await {
                        Ok(_) => String::from("Successfully broadcast message to terraria server"),
                        Err(e) => {
                            tracing::error!("Could not send message to terraria server\n{e:#?}");
                            String::from("There was an issue sending message to terraria server")
                        }
                    };
                    if let Err(e) = discord_request(
                        format!("webhooks/{}/{}", state.client_id, payload.token),
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
                        .content("Successfully requested broadcast of message to terraria server"),
                ))?);
            }
            (_, _, _) => {}
        }
    }
    Ok(json!({}))
}
