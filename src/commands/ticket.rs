use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use serenity::Mentionable;

/// manage the ticket system
#[poise::command(
    slash_command,
    required_permissions = "MANAGE_GUILD",
    subcommands("setup", "config", "close", "add", "remove", "archive", "delete", "transcript")
)]
pub async fn ticket(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("use a subcommand: `setup`, `config`, `close`, `add`, `remove`, `archive`, `delete`, or `transcript`")
        .await?;
    Ok(())
}

/// set up the ticket panel in a channel
#[poise::command(slash_command)]
pub async fn setup(
    ctx: Context<'_>,
    #[description = "channel to post the panel"] panel_channel: serenity::Channel,
    #[description = "category for ticket channels"] category: serenity::Channel,
    #[description = "role that has access to tickets"] support_role: serenity::Role,
    #[description = "channel for ticket logs/transcripts (optional)"] log_channel: Option<serenity::Channel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let db = &ctx.data().db;

    // validate category is actually a category channel
    if !matches!(category, serenity::Channel::Guild(ref ch) if ch.kind == serenity::ChannelType::Category) {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!("{} is not a category channel. please select a category.", category.mention()))
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let category_id = category.id().get();
    let support_role_id = support_role.id.get();
    let panel_channel_id = panel_channel.id().get();
    let log_channel_id = log_channel.as_ref().map(|c| c.id().get());

    let embed = serenity::CreateEmbed::new()
        .title("support tickets")
        .description("need help? click the button below to open a support ticket.\na staff member will assist you shortly.")
        .color(0x5865F2);

    let button = serenity::CreateButton::new("ticket_open")
        .label("open ticket")
        .style(serenity::ButtonStyle::Primary)
        .emoji('🎫');

    let action_row = serenity::CreateActionRow::Buttons(vec![button]);

    let panel_msg = panel_channel
        .id()
        .send_message(
            &ctx,
            serenity::CreateMessage::new()
                .embed(embed)
                .components(vec![action_row]),
        )
        .await?;

    sqlx::query(
        "INSERT INTO ticket_config (guild_id, category_id, support_role_id, panel_channel_id, panel_message_id, log_channel_id) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (guild_id) DO UPDATE SET \
         category_id = $2, support_role_id = $3, panel_channel_id = $4, panel_message_id = $5, log_channel_id = $6",
    )
    .bind(gid)
    .bind(category_id as i64)
    .bind(support_role_id as i64)
    .bind(panel_channel_id as i64)
    .bind(panel_msg.id.get() as i64)
    .bind(log_channel_id.map(|id| id as i64))
    .execute(db)
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!(
                    "✅ ticket panel set up in {}\n\
                     category: {}\n\
                     support role: {}\n\
                     log channel: {}",
                    panel_channel.mention(),
                    category.mention(),
                    support_role.mention(),
                    log_channel
                        .as_ref()
                        .map(|c| c.mention().to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// view or update ticket configuration
#[poise::command(slash_command)]
pub async fn config(
    ctx: Context<'_>,
    #[description = "allow users to close their own tickets (true/false)"] allow_user_close: Option<bool>,
    #[description = "what happens when a ticket is closed: delete or archive"] close_action: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let db = &ctx.data().db;

    let row = sqlx::query_as::<_, (i64, i64, Option<i64>, bool, String)>(
        "SELECT category_id, support_role_id, log_channel_id, allow_user_close, close_action FROM ticket_config WHERE guild_id = $1",
    )
    .bind(gid)
    .fetch_optional(db)
    .await?;

    if row.is_none() {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("no ticket config found. use `/ticket setup` first.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if let Some(allow) = allow_user_close {
        sqlx::query("UPDATE ticket_config SET allow_user_close = $1 WHERE guild_id = $2")
            .bind(allow)
            .bind(gid)
            .execute(db)
            .await?;
    }
    if let Some(ref action) = close_action {
        if action != "delete" && action != "archive" {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("close_action must be `delete` or `archive`.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
        sqlx::query("UPDATE ticket_config SET close_action = $1 WHERE guild_id = $2")
            .bind(action)
            .bind(gid)
            .execute(db)
            .await?;
    }

    let (category_id, support_role_id, log_channel_id, allow_user_close, close_action) =
        sqlx::query_as::<_, (i64, i64, Option<i64>, bool, String)>(
            "SELECT category_id, support_role_id, log_channel_id, allow_user_close, close_action FROM ticket_config WHERE guild_id = $1",
        )
        .bind(gid)
        .fetch_one(db)
        .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("ticket configuration")
                .field("category", format!("<#{}>", category_id), true)
                .field("support role", format!("<@&{}>", support_role_id), true)
                .field(
                    "log channel",
                    log_channel_id
                        .map(|id| format!("<#{}>", id))
                        .unwrap_or_else(|| "none".to_string()),
                    true,
                )
                .field("allow user close", allow_user_close.to_string(), true)
                .field("close action", &close_action, true)
                .color(0x5865F2),
        ),
    )
    .await?;
    Ok(())
}

/// close the current ticket (run inside a ticket channel)
#[poise::command(slash_command)]
pub async fn close(
    ctx: Context<'_>,
    #[description = "save transcript? (default: yes)"] save_transcript: Option<bool>,
    #[description = "override close action: delete or archive (optional)"] action: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    let row = sqlx::query_as::<_, (i64, i64, String, bool, String)>(
        "SELECT t.id, t.creator_id, t.status, c.allow_user_close, c.close_action \
         FROM tickets t JOIN ticket_config c ON t.guild_id = c.guild_id \
         WHERE t.channel_id = $1 AND t.guild_id = $2",
    )
    .bind(channel_id)
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (ticket_id, creator_id, status, allow_user_close, default_action) = match row {
        Some(r) => r,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("this is not a ticket channel.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    if status != "open" {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("this ticket is already closed.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let author_id = ctx.author().id.get() as i64;
    let is_staff = {
        let member = ctx.author_member().await;
        match member {
            Some(m) => {
                let config_role = sqlx::query_scalar::<_, i64>("SELECT support_role_id FROM ticket_config WHERE guild_id = $1")
                    .bind(gid)
                    .fetch_one(db)
                    .await?;
                m.roles.iter().any(|r| r.get() as i64 == config_role)
            }
            None => false,
        }
    };

    if !is_staff && (!allow_user_close || author_id != creator_id) {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("you do not have permission to close this ticket.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let do_transcript = save_transcript.unwrap_or(true);
    let final_action = action.unwrap_or(default_action);

    if do_transcript {
        let guild_name = ctx.guild().map(|g| g.name.clone()).unwrap_or_else(|| "server".to_string());
        match generate_transcript(ctx.serenity_context(), &ctx.channel_id(), &guild_name).await {
            Ok((content, filename)) => {
                let log_channel_id = sqlx::query_scalar::<_, i64>("SELECT log_channel_id FROM ticket_config WHERE guild_id = $1")
                    .bind(gid)
                    .fetch_one(db)
                    .await?;

                if log_channel_id != 0 {
                    let log_ch = serenity::ChannelId::new(log_channel_id as u64);
                    let _ = log_ch
                        .send_message(
                            &ctx,
                            serenity::CreateMessage::new()
                                .content(format!(
                                    "transcript for ticket #{} (opened by <@{}>)",
                                    ticket_id, creator_id
                                ))
                                .add_file(serenity::CreateAttachment::bytes(
                                    content.as_bytes(),
                                    &filename,
                                )),
                        )
                        .await;
                }

                let user = serenity::UserId::new(creator_id as u64);
                let _ = user
                    .dm(
                        &ctx,
                        serenity::CreateMessage::new()
                            .content(format!("transcript for your ticket #{} in **{}**", ticket_id, guild_name))
                            .add_file(serenity::CreateAttachment::bytes(
                                content.as_bytes(),
                                &filename,
                            )),
                    )
                    .await;
            }
            Err(e) => {
                ctx.say(format!("failed to generate transcript: {}", e))
                    .await?;
            }
        }
    }

    sqlx::query("UPDATE tickets SET status = 'closed', closed_at = NOW(), closed_by = $1 WHERE id = $2")
        .bind(author_id)
        .bind(ticket_id)
        .execute(db)
        .await?;

    let action_str = if final_action == "archive" {
        let channels = guild_id.channels(&ctx).await.unwrap_or_default();
        let archive_category = channels
            .values()
            .find(|c| c.kind == serenity::ChannelType::Category && c.name == "archived tickets");

        let archive_cat_id = match archive_category {
            Some(c) => c.id,
            None => {
                let new_cat = guild_id
                    .create_channel(
                        &ctx,
                        serenity::CreateChannel::new("archived tickets")
                            .kind(serenity::ChannelType::Category),
                    )
                    .await?;
                new_cat.id
            }
        };

        let ch = serenity::ChannelId::new(channel_id as u64);
        ch.edit(
            &ctx,
            serenity::EditChannel::new().category(archive_cat_id),
        )
        .await?;

        sqlx::query("UPDATE tickets SET status = 'archived' WHERE id = $1")
            .bind(ticket_id)
            .execute(db)
            .await?;

        "archived"
    } else {
        let ch = serenity::ChannelId::new(channel_id as u64);
        let _ = ch.delete(&ctx).await;
        "deleted"
    };

    if final_action != "delete" {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!(" ticket #{} has been {}.", ticket_id, action_str))
                    .color(0x80F291),
            ),
        )
        .await?;
    }

    Ok(())
}

/// add a user to this ticket
#[poise::command(slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "user to add"] user: serenity::Member,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tickets WHERE channel_id = $1 AND guild_id = $2 AND status = 'open')",
    )
    .bind(channel_id)
    .bind(gid)
    .fetch_one(db)
    .await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("this is not an open ticket channel.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let ch = serenity::ChannelId::new(channel_id as u64);
    ch.edit(
        &ctx,
        serenity::EditChannel::new().permissions(vec![
            serenity::PermissionOverwrite {
                allow: serenity::Permissions::VIEW_CHANNEL
                    | serenity::Permissions::SEND_MESSAGES
                    | serenity::Permissions::READ_MESSAGE_HISTORY,
                deny: serenity::Permissions::empty(),
                kind: serenity::PermissionOverwriteType::Member(user.user.id),
            },
        ]),
    )
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ added {} to this ticket.", user.mention()))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// remove a user from this ticket
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "user to remove"] user: serenity::Member,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tickets WHERE channel_id = $1 AND guild_id = $2 AND status = 'open')",
    )
    .bind(channel_id)
    .bind(gid)
    .fetch_one(db)
    .await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("this is not an open ticket channel.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let ch = serenity::ChannelId::new(channel_id as u64);
    ch.edit(
        &ctx,
        serenity::EditChannel::new().permissions(vec![
            serenity::PermissionOverwrite {
                allow: serenity::Permissions::empty(),
                deny: serenity::Permissions::VIEW_CHANNEL
                    | serenity::Permissions::SEND_MESSAGES
                    | serenity::Permissions::READ_MESSAGE_HISTORY,
                kind: serenity::PermissionOverwriteType::Member(user.user.id),
            },
        ]),
    )
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ removed {} from this ticket.", user.mention()))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// archive the current ticket (move to archive category)
#[poise::command(slash_command)]
pub async fn archive(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    let row = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, status FROM tickets WHERE channel_id = $1 AND guild_id = $2",
    )
    .bind(channel_id)
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (ticket_id, status) = match row {
        Some(r) => r,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("this is not a ticket channel.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    if status == "archived" {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("this ticket is already archived.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let channels = guild_id.channels(&ctx).await.unwrap_or_default();
    let archive_category = channels
        .values()
        .find(|c| c.kind == serenity::ChannelType::Category && c.name == "archived tickets");

    let archive_cat_id = match archive_category {
        Some(c) => c.id,
        None => {
            let new_cat = guild_id
                .create_channel(
                    &ctx,
                    serenity::CreateChannel::new("archived tickets")
                        .kind(serenity::ChannelType::Category),
                )
                .await?;
            new_cat.id
        }
    };

    let ch = serenity::ChannelId::new(channel_id as u64);
    ch.edit(
        &ctx,
        serenity::EditChannel::new().category(archive_cat_id),
    )
    .await?;

    sqlx::query("UPDATE tickets SET status = 'archived', closed_at = NOW(), closed_by = $1 WHERE id = $2")
        .bind(ctx.author().id.get() as i64)
        .bind(ticket_id)
        .execute(db)
        .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ ticket #{} has been archived.", ticket_id))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// delete the current ticket channel
#[poise::command(slash_command)]
pub async fn delete(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    let row = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, status FROM tickets WHERE channel_id = $1 AND guild_id = $2",
    )
    .bind(channel_id)
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (ticket_id, _status) = match row {
        Some(r) => r,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("this is not a ticket channel.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    sqlx::query("UPDATE tickets SET status = 'deleted', closed_at = NOW(), closed_by = $1 WHERE id = $2")
        .bind(ctx.author().id.get() as i64)
        .bind(ticket_id)
        .execute(db)
        .await?;

    let ch = serenity::ChannelId::new(channel_id as u64);
    let _ = ch.delete(&ctx).await;

    Ok(())
}

/// save a transcript of the current ticket without closing it
#[poise::command(slash_command)]
pub async fn transcript(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    let row = sqlx::query_as::<_, (i64, i64)>(
        "SELECT id, creator_id FROM tickets WHERE channel_id = $1 AND guild_id = $2 AND status = 'open'",
    )
    .bind(channel_id)
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (ticket_id, creator_id) = match row {
        Some(r) => r,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("this is not an open ticket channel.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let guild_name = ctx.guild().map(|g| g.name.clone()).unwrap_or_else(|| "server".to_string());
    let (content, filename) =
        generate_transcript(ctx.serenity_context(), &ctx.channel_id(), &guild_name).await?;

    let log_channel_id = sqlx::query_scalar::<_, i64>("SELECT log_channel_id FROM ticket_config WHERE guild_id = $1")
        .bind(gid)
        .fetch_one(db)
        .await?;

    if log_channel_id != 0 {
        let log_ch = serenity::ChannelId::new(log_channel_id as u64);
        let _ = log_ch
            .send_message(
                &ctx,
                serenity::CreateMessage::new()
                    .content(format!(
                        "transcript for ticket #{} (opened by <@{}>)",
                        ticket_id, creator_id
                    ))
                    .add_file(serenity::CreateAttachment::bytes(
                        content.as_bytes(),
                        &filename,
                    )),
            )
            .await;
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ transcript saved for ticket #{}.", ticket_id))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

// ── interaction handlers ──────────────────────────────────────────────

/// Handle the "open ticket" button on the panel embed
pub async fn handle_ticket_open(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    db: &sqlx::PgPool,
) -> Result<(), Error> {
    let guild_id = component
        .guild_id
        .ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let user_id = component.user.id.get() as i64;

    let config = sqlx::query_as::<_, (i64, i64, Option<i64>)>(
        "SELECT category_id, support_role_id, log_channel_id FROM ticket_config WHERE guild_id = $1",
    )
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (_category_id, _support_role_id, _log_channel_id) = match config {
        Some(c) => c,
        None => {
            component
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("ticket system is not configured.")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM tickets WHERE guild_id = $1 AND creator_id = $2 AND status = 'open')",
    )
    .bind(gid)
    .bind(user_id)
    .fetch_one(db)
    .await?;

    if existing {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("you already have an open ticket! please close it before opening a new one.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let modal = serenity::CreateInteractionResponse::Modal(
        serenity::CreateModal::new("ticket_modal", "open a ticket")
            .components(vec![
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "subject",
                        "ticket_subject",
                    )
                    .placeholder("brief description of your issue")
                    .required(true)
                    .max_length(100),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Paragraph,
                        "description",
                        "ticket_description",
                    )
                    .placeholder("tell us more about what you need help with")
                    .required(true)
                    .max_length(2000),
                ),
            ]),
    );

    component.create_response(ctx, modal).await?;
    Ok(())
}

/// Handle the ticket modal submission (creates the channel)
pub async fn handle_ticket_modal(
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
    let user_id = modal.user.id.get() as i64;

    let mut subject = String::new();
    let mut description = String::new();
    for row in &modal.data.components {
        for component in &row.components {
            if let serenity::ActionRowComponent::InputText(input) = component {
                match input.custom_id.as_str() {
                    "ticket_subject" => subject = input.value.clone().unwrap_or_default(),
                    "ticket_description" => description = input.value.clone().unwrap_or_default(),
                    _ => {}
                }
            }
        }
    }

    if subject.is_empty() || description.is_empty() {
        let _ = modal.create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content("please fill in all fields.")
                    .ephemeral(true),
            ),
        ).await;
        return Ok(());
    }

    let config = match sqlx::query_as::<_, (i64, i64)>(
        "SELECT category_id, support_role_id FROM ticket_config WHERE guild_id = $1",
    )
    .bind(gid)
    .fetch_optional(db)
    .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            let _ = modal.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("ticket system is not configured. an admin needs to run `/ticket setup`.")
                        .ephemeral(true),
                ),
            ).await;
            return Ok(());
        }
        Err(e) => {
            tracing::error!("ticket modal db error: {}", e);
            let _ = modal.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("a database error occurred. please try again.")
                        .ephemeral(true),
                ),
            ).await;
            return Ok(());
        }
    };

    let (category_id, support_role_id) = config;

    // validate the category exists and is actually a category
    let cat_id = serenity::ChannelId::new(category_id as u64);
    match cat_id.to_channel(ctx).await {
        Ok(serenity::Channel::Guild(ref ch)) if ch.kind == serenity::ChannelType::Category => {}
        _ => {
            let _ = modal.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("the configured category no longer exists or is invalid. an admin needs to re-run `/ticket setup`.")
                        .ephemeral(true),
                ),
            ).await;
            return Ok(());
        }
    }

    let max_number: Option<i32> = match sqlx::query_scalar("SELECT MAX(number) FROM tickets WHERE guild_id = $1")
        .bind(gid)
        .fetch_one(db)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("ticket number query error: {}", e);
            let _ = modal.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("a database error occurred. please try again.")
                        .ephemeral(true),
                ),
            ).await;
            return Ok(());
        }
    };
    let ticket_number = max_number.unwrap_or(0) + 1;

    let guild = serenity::GuildId::new(guild_id.get());
    let channel_name = format!("ticket-{}", ticket_number);
    let ticket_channel = match guild
        .create_channel(
            ctx,
            serenity::CreateChannel::new(&channel_name)
                .kind(serenity::ChannelType::Text)
                .category(serenity::ChannelId::new(category_id as u64))
                .permissions(vec![
                    serenity::PermissionOverwrite {
                        allow: serenity::Permissions::empty(),
                        deny: serenity::Permissions::VIEW_CHANNEL,
                        kind: serenity::PermissionOverwriteType::Role(serenity::RoleId::new(guild_id.get())),
                    },
                    serenity::PermissionOverwrite {
                        allow: serenity::Permissions::VIEW_CHANNEL
                            | serenity::Permissions::SEND_MESSAGES
                            | serenity::Permissions::READ_MESSAGE_HISTORY,
                        deny: serenity::Permissions::empty(),
                        kind: serenity::PermissionOverwriteType::Member(modal.user.id),
                    },
                    serenity::PermissionOverwrite {
                        allow: serenity::Permissions::VIEW_CHANNEL
                            | serenity::Permissions::SEND_MESSAGES
                            | serenity::Permissions::READ_MESSAGE_HISTORY,
                        deny: serenity::Permissions::empty(),
                        kind: serenity::PermissionOverwriteType::Role(
                            serenity::RoleId::new(support_role_id as u64),
                        ),
                    },
                ]),
        )
        .await
    {
        Ok(ch) => ch,
        Err(e) => {
            tracing::error!("ticket channel creation error: {}", e);
            let _ = modal.create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("failed to create ticket channel: {}", e))
                        .ephemeral(true),
                ),
            ).await;
            return Ok(());
        }
    };

    if let Err(e) = sqlx::query(
        "INSERT INTO tickets (guild_id, channel_id, creator_id, number) VALUES ($1, $2, $3, $4)",
    )
    .bind(gid)
    .bind(ticket_channel.id.get() as i64)
    .bind(user_id)
    .bind(ticket_number)
    .execute(db)
    .await
    {
        tracing::error!("ticket insert error: {}", e);
        let _ = modal.create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content("failed to save ticket to database. please try again.")
                    .ephemeral(true),
            ),
        ).await;
        return Ok(());
    }

    let embed = serenity::CreateEmbed::new()
        .title(format!("ticket #{}", ticket_number))
        .description(format!(
            "**subject:** {}\n\n{}\n\n\
             <@&{}> has been notified. a staff member will be with you shortly.\n\
             use the button below to close this ticket.",
            subject, description, support_role_id
        ))
        .color(0x5865F2)
        .timestamp(chrono::Utc::now());

    let close_button = serenity::CreateButton::new("ticket_close")
        .label("close ticket")
        .style(serenity::ButtonStyle::Danger)
        .emoji('🔒');

    let action_row = serenity::CreateActionRow::Buttons(vec![close_button]);

    let _ = ticket_channel
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .embed(embed)
                .content(format!(
                    "<@{}> <@&{}>",
                    modal.user.id, support_role_id
                ))
                .components(vec![action_row]),
        )
        .await;

    let _ = modal
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(format!("ticket created: {}", ticket_channel.mention()))
                    .ephemeral(true),
            ),
        )
        .await;

    Ok(())
}

/// Handle the "close ticket" button inside a ticket channel
pub async fn handle_ticket_close(
    ctx: &serenity::Context,
    component: &serenity::ComponentInteraction,
    db: &sqlx::PgPool,
) -> Result<(), Error> {
    let guild_id = component
        .guild_id
        .ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let channel_id = component.channel_id.get() as i64;
    let user_id = component.user.id.get() as i64;

    let row = sqlx::query_as::<_, (i64, i64, String, bool, String)>(
        "SELECT t.id, t.creator_id, t.status, c.allow_user_close, c.close_action \
         FROM tickets t JOIN ticket_config c ON t.guild_id = c.guild_id \
         WHERE t.channel_id = $1 AND t.guild_id = $2",
    )
    .bind(channel_id)
    .bind(gid)
    .fetch_optional(db)
    .await?;

    let (ticket_id, creator_id, status, allow_user_close, close_action) = match row {
        Some(r) => r,
        None => {
            component
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("this is not a ticket channel.")
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
    };

    if status != "open" {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("this ticket is already closed.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let is_staff = {
        let config_role = sqlx::query_scalar::<_, i64>("SELECT support_role_id FROM ticket_config WHERE guild_id = $1")
            .bind(gid)
            .fetch_one(db)
            .await?;
        let member = serenity::GuildId::new(gid as u64)
            .member(ctx, component.user.id)
            .await;
        match member {
            Ok(m) => m.roles.iter().any(|r| r.get() as i64 == config_role),
            Err(_) => false,
        }
    };

    if !is_staff && (!allow_user_close || user_id != creator_id) {
        component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("you do not have permission to close this ticket.")
                        .ephemeral(true),
                ),
            )
            .await?;
        return Ok(());
    }

    let guild_name = serenity::GuildId::new(gid as u64)
        .to_partial_guild(ctx)
        .await
        .map(|g| g.name)
        .unwrap_or_else(|_| "server".to_string());

    let ch = serenity::ChannelId::new(channel_id as u64);
    match generate_transcript(ctx, &ch, &guild_name).await {
        Ok((content, filename)) => {
            let log_channel_id = sqlx::query_scalar::<_, i64>("SELECT log_channel_id FROM ticket_config WHERE guild_id = $1")
                .bind(gid)
                .fetch_one(db)
                .await?;

            if log_channel_id != 0 {
                let log_ch = serenity::ChannelId::new(log_channel_id as u64);
                let _ = log_ch
                    .send_message(
                        ctx,
                        serenity::CreateMessage::new()
                            .content(format!(
                                "transcript for ticket #{} (opened by <@{}>)",
                                ticket_id, creator_id
                            ))
                            .add_file(serenity::CreateAttachment::bytes(
                                content.as_bytes(),
                                &filename,
                            )),
                    )
                    .await;
            }

            let _ = component
                .user
                .dm(
                    ctx,
                    serenity::CreateMessage::new()
                        .content(format!("transcript for your ticket #{} in **{}**", ticket_id, guild_name))
                        .add_file(serenity::CreateAttachment::bytes(
                            content.as_bytes(),
                            &filename,
                        )),
                )
                .await;
        }
        Err(e) => {
            tracing::error!("failed to generate transcript for ticket #{}: {}", ticket_id, e);
        }
    }

    sqlx::query("UPDATE tickets SET status = 'closed', closed_at = NOW(), closed_by = $1 WHERE id = $2")
        .bind(user_id)
        .bind(ticket_id)
        .execute(db)
        .await?;

    if close_action == "archive" {
        let channels = serenity::GuildId::new(gid as u64)
            .channels(ctx)
            .await
            .unwrap_or_default();

        let archive_category = channels
            .values()
            .find(|c| c.kind == serenity::ChannelType::Category && c.name == "archived tickets");

        let archive_cat_id = match archive_category {
            Some(c) => c.id,
            None => {
                let new_cat = serenity::GuildId::new(gid as u64)
                    .create_channel(
                        ctx,
                        serenity::CreateChannel::new("archived tickets")
                            .kind(serenity::ChannelType::Category),
                    )
                    .await?;
                new_cat.id
            }
        };

        let _ = ch
            .edit(ctx, serenity::EditChannel::new().category(archive_cat_id))
            .await;

        sqlx::query("UPDATE tickets SET status = 'archived' WHERE id = $1")
            .bind(ticket_id)
            .execute(db)
            .await?;

        let _ = component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!(
                            "ticket #{} has been archived by {}. transcript saved.",
                            ticket_id,
                            component.user.mention()
                        ))
                        .components(vec![]),
                ),
            )
            .await;
    } else {
        let _ = component
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!(
                            "ticket #{} closed by {}. deleting channel...",
                            ticket_id,
                            component.user.mention()
                        ))
                        .components(vec![]),
                ),
            )
            .await;

        let _ = ch.delete(ctx).await;
    }

    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────

async fn generate_transcript(
    ctx: &serenity::Context,
    channel_id: &serenity::ChannelId,
    guild_name: &str,
) -> Result<(String, String), Error> {
    let mut messages = channel_id
        .messages(ctx, serenity::GetMessages::new().limit(100))
        .await?;
    let mut before = messages.last().map(|m| m.id);

    while let Some(before_id) = before {
        let batch = channel_id
            .messages(
                ctx,
                serenity::GetMessages::new().before(before_id).limit(100),
            )
            .await?;
        if batch.is_empty() {
            break;
        }
        before = batch.last().map(|m| m.id);
        messages.extend(batch);
    }

    messages.reverse();

    let mut html = format!(
        "<!DOCTYPE html>\n<html>\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <title>transcript — {}</title>\n\
         <style>\n\
         body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; \
                background: #2c2f33; color: #dcddde; padding: 20px; margin: 0; }}\n\
         .header {{ text-align: center; margin-bottom: 24px; padding-bottom: 16px; \
                    border-bottom: 1px solid #40444b; }}\n\
         .header h1 {{ color: #fff; font-size: 18px; margin: 0; }}\n\
         .header p {{ color: #72767d; font-size: 12px; margin: 4px 0 0 0; }}\n\
         .msg {{ display: flex; gap: 12px; padding: 4px 0; }}\n\
         .msg:hover {{ background: #2f3136; border-radius: 4px; }}\n\
         .avatar {{ width: 40px; height: 40px; border-radius: 50%; flex-shrink: 0; }}\n\
         .content {{ flex: 1; min-width: 0; }}\n\
         .author {{ font-weight: 600; color: #fff; font-size: 14px; }}\n\
         .timestamp {{ color: #72767d; font-size: 11px; margin-left: 8px; }}\n\
         .text {{ color: #dcddde; font-size: 14px; line-height: 1.4; \
                  word-wrap: break-word; margin-top: 2px; }}\n\
         .attachment {{ color: #00aff4; font-size: 13px; margin-top: 4px; }}\n\
         </style>\n</head>\n<body>\n\
         <div class=\"header\">\n\
         <h1>transcript — {}</h1>\n\
         <p>{} messages</p>\n\
         </div>\n",
        guild_name,
        guild_name,
        messages.len(),
    );

    for msg in &messages {
        let avatar_url = msg.author.face();
        let timestamp = msg
            .timestamp
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let content = html_escape(&msg.content);
        let attachments = if msg.attachments.is_empty() {
            String::new()
        } else {
            let names: Vec<&str> = msg.attachments.iter().map(|a| a.filename.as_str()).collect();
            format!(
                "<div class=\"attachment\">📎 {}</div>",
                names.join(", ")
            )
        };

        html.push_str(&format!(
            "<div class=\"msg\">\n\
             <img class=\"avatar\" src=\"{}\" alt=\"{}\">\n\
             <div class=\"content\">\n\
             <span class=\"author\">{}</span>\n\
             <span class=\"timestamp\">{}</span>\n\
             <div class=\"text\">{}{}</div>\n\
             </div>\n</div>\n",
            avatar_url,
            html_escape(&msg.author.name),
            html_escape(&msg.author.name),
            timestamp,
            content,
            attachments,
        ));
    }

    html.push_str("</body>\n</html>");

    let filename = format!(
        "transcript-{}.html",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );

    Ok((html, filename))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
