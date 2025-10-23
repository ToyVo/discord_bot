use {
    crate::{
        error::AppError,
        server::models::{GameServer, MessageType},
        state::AppState,
    },
    anyhow::Context,
    axum::http::header,
    reqwest::Method,
    serde::Serialize,
    serde_json::Value,
    serenity::builder::CreateMessage,
};

#[poise::command(slash_command)]
pub async fn minecraft_geyser(
    ctx: crate::state::Context<'_>,
    // TODO: limit to stop and restart
    #[description = "The action to perform"] _action: String,
) -> Result<(), AppError> {
    let state = ctx.data().lock().await;
    ctx.say(format!(
        "Hello! The shared state value is: {}",
        state.base_url
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn minecraft_modded(
    ctx: crate::state::Context<'_>,
    // TODO: limit to stop and restart
    #[description = "The action to perform"] _action: String,
) -> Result<(), AppError> {
    let state = ctx.data().lock().await;
    ctx.say(format!(
        "Hello! The shared state value is: {}",
        state.base_url
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn terraria(
    ctx: crate::state::Context<'_>,
    // TODO: limit to stop, restart, and send message (with message)
    #[description = "The action to perform"] _action: String,
) -> Result<(), AppError> {
    let state = ctx.data().lock().await;
    ctx.say(format!(
        "Hello! The shared state value is: {}",
        state.base_url
    ))
    .await?;
    Ok(())
}

pub async fn discord_request<S: AsRef<str>, T: Serialize + ?Sized>(
    endpoint: S,
    method: Method,
    body: Option<&T>,
    state: &AppState,
) -> Result<Option<Value>, AppError> {
    let url = format!("https://discord.com/api/v10/{}", endpoint.as_ref());

    let mut builder = reqwest::Client::new()
        .request(method.clone(), url.as_str())
        .header(
            header::AUTHORIZATION.as_str(),
            format!("Bot {}", { state.discord_token.as_str() }),
        )
        .header(header::USER_AGENT.as_str(), state.user_agent.as_str());

    if let Some(b) = body {
        builder = builder
            .header(
                header::CONTENT_TYPE.as_str(),
                mime::APPLICATION_JSON.as_ref(),
            )
            .json(b);
    }

    let response = builder.send().await?;

    tracing::debug!("response from {method} {url}: {response:?}");

    let content_type = response.headers().get(header::CONTENT_TYPE.as_str());
    if let Some(content_type) = content_type {
        if content_type.to_str().unwrap() == mime::APPLICATION_JSON.as_ref() {
            let body = response.json::<Value>().await?;
            tracing::debug!("response body from {method} {url}: {body}");
            return Ok(Some(body));
        }
    }

    Ok(None)
}

pub async fn create_message<S: AsRef<str>>(
    payload: CreateMessage,
    channel_id: S,
    state: &AppState,
) -> Result<Value, AppError> {
    let endpoint = format!("channels/{}/messages", channel_id.as_ref());
    let response = discord_request(endpoint, Method::POST, Some(&payload), state).await?;
    let json = response.context("Response not found from creating message")?;
    tracing::info!(
        "Message created {}",
        json.get("id")
            .context("id not found")?
            .as_str()
            .context("failed to parse as str")?
    );
    Ok(json)
}

pub async fn send_message<S: AsRef<str>>(
    message: &String,
    _message_type: MessageType,
    _server: GameServer,
    _channel_id: S,
    _state: &AppState,
) -> Result<(), AppError> {
    tracing::info!("{message}");
    // let created_message = create_message(
    //     CreateMessage::new()
    //         .content(message)
    //         .flags(MessageFlags::SUPPRESS_NOTIFICATIONS),
    //     &channel_id,
    //     state,
    // )
    //     .await?;

    // match state.db.select((DBCollection::DiscordMessages.to_string(), server.to_string())).await {
    //     Ok(Some(data)) => {
    //         let data: DiscordMessage = data;
    //         delete_message(
    //             data.discord_message_id.as_str(),
    //             channel_id.as_ref(),
    //             state,
    //         )
    //             .await
    //     }
    //     Err(e) => {
    //         tracing::error!("Error getting DiscordMessage from DB: {e}");
    //         Ok(())
    //     },
    //     _ => Ok(()),
    // }?;

    // let _: Option<DiscordMessage> = state
    //     .db
    //     .upsert((DBCollection::DiscordMessages.to_string(), server.to_string()))
    //     .content(DiscordMessage {
    //         game: server,
    //         discord_message_id: created_message
    //             .get("id")
    //             .context("Could not find id in response")?
    //             .as_str()
    //             .context("could not parse as str")?
    //             .to_string(),
    //         message_type,
    //     })
    //     .await?;

    Ok(())
}

pub async fn delete_message<S: AsRef<str>>(
    message_id: S,
    channel_id: S,
    state: &AppState,
) -> Result<(), AppError> {
    let endpoint = format!(
        "channels/{}/messages/{}",
        channel_id.as_ref(),
        message_id.as_ref()
    );
    discord_request(endpoint, Method::DELETE, None::<&str>, state).await?;
    Ok(())
}
