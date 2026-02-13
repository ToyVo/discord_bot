use crate::{
    error::AppError,
    models::{NewReactionMessage, NewSelfAssignableRole},
    queries::{
        create_reaction_message, create_self_assignable_role, delete_self_assignable_role, get_reaction_message, get_self_assignable_roles
    },
    schema::self_assignable_roles,
};
use rust_i18n::t;
use {
    crate::state::{AppState, MessageType},
    poise::serenity_prelude as serenity,
    std::sync::Arc,
    tokio::sync::Mutex,
};

#[poise::command(
    slash_command,
    subcommands("minecraft_geyser_restart", "minecraft_geyser_stop")
)]
pub async fn minecraft_geyser(_ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    unreachable!()
}

#[poise::command(slash_command)]
pub async fn minecraft_geyser_restart(ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    ctx.say(String::from("Restarting Java/Bedrock Minecraft Server"))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn minecraft_geyser_stop(ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    ctx.say(String::from("Stopping Java/Bedrock Minecraft Server"))
        .await?;
    Ok(())
}

#[poise::command(
    slash_command,
    subcommands("minecraft_modded_restart", "minecraft_modded_stop")
)]
pub async fn minecraft_modded(_ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    unreachable!()
}

#[poise::command(slash_command)]
pub async fn minecraft_modded_restart(ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    ctx.say(String::from("Restarting Modded Minecraft Server"))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn minecraft_modded_stop(ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    ctx.say(String::from("Stopping Modded Minecraft Server"))
        .await?;
    Ok(())
}

#[poise::command(
    slash_command,
    subcommands("terraria_broadcast_message", "terraria_restart", "terraria_stop")
)]
pub async fn terraria(_ctx: crate::state::Context<'_>) -> Result<(), AppError> {
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
pub async fn terraria_restart(ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    ctx.say(String::from("Restarting Terraria Server")).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn terraria_stop(ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    ctx.say(String::from("Stopping Terraria Server")).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn game_roles(ctx: crate::state::Context<'_>) -> Result<(), AppError> {
    if let Some(guild) = ctx.partial_guild().await {
        let self_assignable_roles =
            get_self_assignable_roles(pool, u64::from(guild.id).to_string()).await?;
        let message = self_assignable_roles
            .iter()
            .map(|sar| format!("{} = <@&{}>", sar.emoji, sar.role_id))
            .collect::<Vec<_>>()
            .join("\n");
        let sent_message = ctx
            .say(format!("{}\n{}", t!("roles.intro"), message,))
            .await?;
        let sent_message_id = sent_message.message().await?.id;
        let new_message = NewReactionMessage {
            guild_id: u64::from(guild.id).to_string(),
            message_id: u64::from(sent_message_id).to_string(),
            message_type: MessageType::RoleAssigner.to_string(),
        };
        let new_message = create_reaction_message(pool, new_message).await?;
    }
    Ok(())
}

#[poise::command(slash_command)]
pub async fn register_self_assignable_role(
    ctx: crate::state::Context<'_>,
    #[description = "Pick a role"] role: serenity::RoleId,
    #[description = "Reaction emoji"] emoji: String,
) -> Result<(), AppError> {
    let new_role = NewSelfAssignableRole {
        emoji: emoji.clone(),
        guild_id: u64::from(ctx.guild_id().unwrap()).to_string(),
        role_id: u64::from(role).to_string(),
    };
    let new_role = create_self_assignable_role(pool, new_role).await?;
    ctx.say(format!(
        "Registered self-assignable role: <@&{}> with emoji {}",
        role, emoji
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn deregister_self_assignable_role(
    ctx: crate::state::Context<'_>,
    #[description = "Pick a role"] role: serenity::RoleId,
) -> Result<(), AppError> {
    delete_self_assignable_role(
        pool,
        u64::from(ctx.guild_id().unwrap()).to_string(),
        u64::from(role).to_string(),
    )
    .await?;
    ctx.say(format!("Deregistered self-assignable role: <@&{}>", role))
        .await?;
    Ok(())
}

pub async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Arc<Mutex<AppState>>, AppError>,
    data: &Arc<Mutex<AppState>>,
) -> Result<(), AppError> {
    match event {
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            let message = get_reaction_message(pool, add_reaction.guild_id, add_reaction.message_id).await?;
            let data = data.lock().await.clone();
            match data.message_ids.get(&u64::from(add_reaction.message_id)) {
                Some(&MessageType::RoleAssigner) => {
                    if let serenity::ReactionType::Unicode(emoji) = add_reaction.clone().emoji {
                        if let Some(role_id) = data.self_assignable_roles.get(&emoji) {
                            if let Some(user_id) = add_reaction.user_id {
                                if let Some(guild_id) = add_reaction.guild_id {
                                    add_reaction
                                        .channel_id
                                        .say(
                                            &ctx.http,
                                            &format!(
                                                "You have been assigned to the role: <@&{}>",
                                                role_id
                                            ),
                                        )
                                        .await?;
                                    let member = guild_id.member(&ctx.http, user_id).await?;
                                    // TODO: this never resolves, the following trace is never printed
                                    member.add_role(&ctx.http, *role_id).await?;
                                    tracing::info!(
                                        "Added Role ID: {:?} to user: {:?}",
                                        role_id,
                                        add_reaction.user_id
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        serenity::FullEvent::ReactionRemove { removed_reaction } => {
            let data = data.lock().await.clone();
            match data
                .message_ids
                .get(&u64::from(removed_reaction.message_id))
            {
                Some(&MessageType::RoleAssigner) => {
                    if let serenity::ReactionType::Unicode(emoji) = removed_reaction.clone().emoji {
                        if let Some(role_id) = data.self_assignable_roles.get(&emoji) {
                            if let Some(user_id) = removed_reaction.user_id {
                                if let Some(guild_id) = removed_reaction.guild_id {
                                    removed_reaction
                                        .channel_id
                                        .say(
                                            &ctx.http,
                                            &format!(
                                                "You have been removed from the role: <@&{}>",
                                                role_id
                                            ),
                                        )
                                        .await?;
                                    let member = guild_id.member(&ctx.http, user_id).await?;
                                    // TODO this never resolves, the following trace is never printed
                                    member.remove_role(&ctx.http, *role_id).await?;
                                    tracing::info!(
                                        "Removed Role ID: {:?} from user: {:?}",
                                        role_id,
                                        removed_reaction.user_id
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}
