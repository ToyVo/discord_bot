use crate::error::AppError;

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
