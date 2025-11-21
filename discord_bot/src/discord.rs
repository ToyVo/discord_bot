use crate::error::AppError;

#[poise::command(slash_command, subcommands("minecraft_geyser_restart", "minecraft_geyser_stop"))]
pub async fn minecraft_geyser(
    _ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    unreachable!()
}

#[poise::command(slash_command)]
pub async fn minecraft_geyser_restart(
    ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    ctx.say(String::from(
        "Restarting Java/Bedrock Minecraft Server"
    ))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn minecraft_geyser_stop(
    ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    ctx.say(String::from(
        "Stopping Java/Bedrock Minecraft Server"
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command, subcommands("minecraft_modded_restart", "minecraft_modded_stop"))]
pub async fn minecraft_modded(
    _ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    unreachable!()
}

#[poise::command(slash_command)]
pub async fn minecraft_modded_restart(
    ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    ctx.say(String::from(
        "Restarting Modded Minecraft Server"
    ))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn minecraft_modded_stop(
    ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    ctx.say(String::from(
        "Stopping Modded Minecraft Server"
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command, subcommands("terraria_broadcast_message", "terraria_restart", "terraria_stop"))]
pub async fn terraria(
    _ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    unreachable!()
}

#[poise::command(slash_command)]
pub async fn terraria_broadcast_message(
    ctx: crate::state::Context<'_>,
    #[description = "The message to broadcast"] message: String,
) -> Result<(), AppError> {
    ctx.say(format!(
        "Broadcasting message: `{}` to Terraria Server",
        message
    ))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn terraria_restart(
    ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    ctx.say(String::from(
        "Restarting Terraria Server"
    ))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn terraria_stop(
    ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    ctx.say(String::from(
        "Stopping Terraria Server"
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn game_roles(
    ctx: crate::state::Context<'_>,
) -> Result<(), AppError> {
    if let Some(guild) = ctx.partial_guild().await {
        ctx.say(format!(
            "Hello! The guild has the following roles: {:?}",
            guild.roles
        ))
            .await?;
    }
    Ok(())
}
