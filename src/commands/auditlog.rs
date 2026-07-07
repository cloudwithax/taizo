use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use serenity::Mentionable;

/// manage the audit log system
#[poise::command(
    slash_command,
    required_permissions = "MANAGE_GUILD",
    subcommands("setup", "config", "disable", "test")
)]
pub async fn auditlog(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("use a subcommand: `setup`, `config`, `disable`, or `test`")
        .await?;
    Ok(())
}

/// set up audit logging for this server
#[poise::command(slash_command)]
pub async fn setup(
    ctx: Context<'_>,
    #[description = "channel to send audit logs (auto-created if omitted)"] channel: Option<serenity::Channel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let db = &ctx.data().db;

    let log_channel_id = match channel {
        Some(ref ch) => {
            if !matches!(ch, serenity::Channel::Guild(ref gc) if gc.kind == serenity::ChannelType::Text) {
                ctx.send(
                    poise::CreateReply::default().embed(
                        serenity::CreateEmbed::new()
                            .description(format!("{} is not a text channel. please select a text channel.", ch.mention()))
                            .color(0xF28080),
                    ),
                )
                .await?;
                return Ok(());
            }
            ch.id().get()
        }
        None => {
            let ch = guild_id
                .create_channel(
                    &ctx,
                    serenity::CreateChannel::new("audit-log")
                        .kind(serenity::ChannelType::Text)
                        .topic("server audit log — do not delete")
                        .permissions(vec![
                            serenity::PermissionOverwrite {
                                allow: serenity::Permissions::empty(),
                                deny: serenity::Permissions::VIEW_CHANNEL,
                                kind: serenity::PermissionOverwriteType::Role(serenity::RoleId::new(guild_id.get())),
                            },
                        ]),
                )
                .await?;
            ch.id.get()
        }
    };

    sqlx::query(
        "INSERT INTO audit_log_config (guild_id, channel_id) VALUES ($1, $2) \
         ON CONFLICT (guild_id) DO UPDATE SET channel_id = $2",
    )
    .bind(gid)
    .bind(log_channel_id as i64)
    .execute(db)
    .await?;

    let msg = if channel.is_some() {
        format!("✅ audit log set up in <#{}>", log_channel_id)
    } else {
        format!("✅ audit log channel created: <#{}>\naudit logging is now active with all defaults enabled.", log_channel_id)
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(&msg)
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// view or update audit log configuration
#[poise::command(slash_command)]
pub async fn config(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let db = &ctx.data().db;

    let row = sqlx::query_as::<_, (i64, bool, bool, bool, bool, bool, bool)>(
        "SELECT channel_id, log_messages, log_members, log_moderation, log_channels, log_roles, log_voice \
         FROM audit_log_config WHERE guild_id = $1",
    )
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (channel_id, log_messages, log_members, log_moderation, log_channels, log_roles, log_voice) = match row {
        Some(r) => r,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("no audit log config found. use `/auditlog setup` first.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let toggle = |on: bool| if on { "✅ enabled" } else { "❌ disabled" };

    let embed = serenity::CreateEmbed::new()
        .title("audit log configuration")
        .field("channel", format!("<#{}>", channel_id), false)
        .field("message events", toggle(log_messages), true)
        .field("member events", toggle(log_members), true)
        .field("moderation actions", toggle(log_moderation), true)
        .field("channel changes", toggle(log_channels), true)
        .field("role changes", toggle(log_roles), true)
        .field("voice events", toggle(log_voice), true)
        .color(0x5865F2);

    let edit_btn = serenity::CreateButton::new("auditlog_edit_config")
        .label("edit config")
        .style(serenity::ButtonStyle::Primary)
        .emoji('⚙');

    let action_row = serenity::CreateActionRow::Buttons(vec![edit_btn]);

    ctx.send(
        poise::CreateReply::default()
            .embed(embed)
            .components(vec![action_row]),
    )
    .await?;
    Ok(())
}

/// disable audit logging for this server
#[poise::command(slash_command)]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let db = &ctx.data().db;

    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM audit_log_config WHERE guild_id = $1)")
        .bind(gid)
        .fetch_one(db)
        .await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("no audit log config found. use `/auditlog setup` first.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("DELETE FROM audit_log_config WHERE guild_id = $1")
        .bind(gid)
        .execute(db)
        .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description("✅ audit logging has been disabled.")
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// send a test embed to the audit log channel
#[poise::command(slash_command)]
pub async fn test(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let db = &ctx.data().db;

    let channel_id = sqlx::query_scalar::<_, i64>("SELECT channel_id FROM audit_log_config WHERE guild_id = $1")
        .bind(gid)
        .fetch_optional(db)
        .await?;

    let channel_id = match channel_id {
        Some(id) => id,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("no audit log config found. use `/auditlog setup` first.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let ch = serenity::ChannelId::new(channel_id as u64);
    ch.send_message(
        &ctx,
        serenity::CreateMessage::new().embed(
            serenity::CreateEmbed::new()
                .title("audit log test")
                .description("this is a test message. if you can see this, audit logging is working correctly.")
                .color(0x5865F2)
                .timestamp(chrono::Utc::now()),
        ),
    )
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ test embed sent to <#{}>.", channel_id))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

// ── interaction handler ──────────────────────────────────────────────

/// Handle the "edit config" button — opens a modal for toggling log categories
pub async fn handle_auditlog_edit_config(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    db: &sqlx::PgPool,
) -> Result<(), Error> {
    let guild_id = component.guild_id.ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;

    let row = sqlx::query_as::<_, (bool, bool, bool, bool, bool, bool)>(
        "SELECT log_messages, log_members, log_moderation, log_channels, log_roles, log_voice \
         FROM audit_log_config WHERE guild_id = $1",
    )
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (messages, members, moderation, channels, roles, voice) = match row {
        Some(r) => r,
        None => {
            component
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("no audit log config found.")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let fmt = |on: bool| if on { "on" } else { "off" };

    let modal = serenity::CreateInteractionResponse::Modal(
        serenity::CreateModal::new("auditlog_modal", "audit log settings")
            .components(vec![
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "log messages (on/off)",
                        "audit_messages",
                    )
                    .placeholder(fmt(messages))
                    .value(fmt(messages))
                    .required(true)
                    .max_length(3),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "log members (on/off)",
                        "audit_members",
                    )
                    .placeholder(fmt(members))
                    .value(fmt(members))
                    .required(true)
                    .max_length(3),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "log moderation (on/off)",
                        "audit_moderation",
                    )
                    .placeholder(fmt(moderation))
                    .value(fmt(moderation))
                    .required(true)
                    .max_length(3),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "log channels (on/off)",
                        "audit_channels",
                    )
                    .placeholder(fmt(channels))
                    .value(fmt(channels))
                    .required(true)
                    .max_length(3),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "log roles (on/off)",
                        "audit_roles",
                    )
                    .placeholder(fmt(roles))
                    .value(fmt(roles))
                    .required(true)
                    .max_length(3),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "log voice (on/off)",
                        "audit_voice",
                    )
                    .placeholder(fmt(voice))
                    .value(fmt(voice))
                    .required(true)
                    .max_length(3),
                ),
            ]),
    );

    component.create_response(ctx, modal).await?;
    Ok(())
}

/// Handle the audit log modal submission
pub async fn handle_auditlog_modal(
    ctx: &serenity::Context,
    modal: &serenity::ModalInteraction,
    db: &sqlx::PgPool,
) -> Result<(), Error> {
    let guild_id = match modal.guild_id {
        Some(g) => g,
        None => {
            let _ = modal.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("this can only be used in a server.")
                        .ephemeral(true),
                ),
            ).await;
            return Ok(());
        }
    };
    let gid = guild_id.get() as i64;

    let mut values = std::collections::HashMap::new();
    for row in &modal.data.components {
        for component in &row.components {
            if let serenity::ActionRowComponent::InputText(input) = component {
                values.insert(input.custom_id.clone(), input.value.clone().unwrap_or_default());
            }
        }
    }

    let parse = |key: &str| -> bool {
        values.get(key).map(|v| v.to_lowercase() == "on").unwrap_or(false)
    };

    let log_messages = parse("audit_messages");
    let log_members = parse("audit_members");
    let log_moderation = parse("audit_moderation");
    let log_channels = parse("audit_channels");
    let log_roles = parse("audit_roles");
    let log_voice = parse("audit_voice");

    sqlx::query(
        "UPDATE audit_log_config SET log_messages = $1, log_members = $2, log_moderation = $3, \
         log_channels = $4, log_roles = $5, log_voice = $6 WHERE guild_id = $7",
    )
    .bind(log_messages)
    .bind(log_members)
    .bind(log_moderation)
    .bind(log_channels)
    .bind(log_roles)
    .bind(log_voice)
    .bind(gid)
    .execute(db)
    .await?;

    let toggle = |on: bool| if on { "✅" } else { "❌" };

    let embed = serenity::CreateEmbed::new()
        .title("audit log config updated")
        .field("messages", toggle(log_messages), true)
        .field("members", toggle(log_members), true)
        .field("moderation", toggle(log_moderation), true)
        .field("channels", toggle(log_channels), true)
        .field("roles", toggle(log_roles), true)
        .field("voice", toggle(log_voice), true)
        .color(0x80F291);

    let _ = modal.create_response(
        ctx,
        serenity::CreateInteractionResponse::UpdateMessage(
            serenity::CreateInteractionResponseMessage::new()
                .embed(embed)
                .components(vec![]),
        ),
    ).await;

    Ok(())
}

// ── logging helpers ──────────────────────────────────────────────────

/// Check if a guild has audit logging enabled for a specific category and return the channel id
pub async fn get_log_channel(db: &sqlx::PgPool, guild_id: i64, category: &str) -> Option<i64> {
    let col = match category {
        "messages" => "log_messages",
        "members" => "log_members",
        "moderation" => "log_moderation",
        "channels" => "log_channels",
        "roles" => "log_roles",
        "voice" => "log_voice",
        _ => return None,
    };

    let query = format!(
        "SELECT channel_id FROM audit_log_config WHERE guild_id = $1 AND {} = true",
        col
    );

    sqlx::query_scalar::<_, i64>(&query)
        .bind(guild_id)
        .fetch_optional(db)
        .await
        .unwrap_or(None)
}

/// Send an audit log embed to the configured channel
pub async fn log_event(
    http: &serenity::Http,
    db: &sqlx::PgPool,
    guild_id: i64,
    category: &str,
    embed: serenity::CreateEmbed,
) {
    if let Some(channel_id) = get_log_channel(db, guild_id, category).await {
        let ch = serenity::ChannelId::new(channel_id as u64);
        let _ = ch.send_message(http, serenity::CreateMessage::new().embed(embed)).await;
    }
}
