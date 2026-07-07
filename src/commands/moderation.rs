use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;
use rand::seq::SliceRandom;

/// ban a member from the server
#[poise::command(slash_command, required_permissions = "BAN_MEMBERS")]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "user to ban"] user: serenity::Member,
    #[description = "reason for the ban"] reason: Option<String>,
) -> Result<(), Error> {
    let reason = reason.unwrap_or_else(|| "no reason provided".to_string());
    let name = user.user.name.clone();

    user.ban_with_reason(&ctx, 0, &reason).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("🛑 banned **{}** — {}", name, reason))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// kick a member from the server
#[poise::command(slash_command, required_permissions = "KICK_MEMBERS")]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "user to kick"] user: serenity::Member,
    #[description = "reason for the kick"] reason: Option<String>,
) -> Result<(), Error> {
    let reason = reason.unwrap_or_else(|| "no reason provided".to_string());
    let name = user.user.name.clone();

    user.kick_with_reason(&ctx, &reason).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("🛑 kicked **{}** — {}", name, reason))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// mute a member (timeout)
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
pub async fn mute(
    ctx: Context<'_>,
    #[description = "user to mute"] mut user: serenity::Member,
    #[description = "duration in minutes"] minutes: u64,
    #[description = "reason for the mute"] reason: Option<String>,
) -> Result<(), Error> {
    let reason = reason.unwrap_or_else(|| "no reason provided".to_string());
    let name = user.user.name.clone();
    let duration = std::time::Duration::from_secs(minutes * 60);
    let timestamp = {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            + duration;
        let dt = chrono::DateTime::from_timestamp(secs.as_secs() as i64, 0)
            .ok_or("invalid timestamp")?;
        dt.to_rfc3339()
    };

    user.edit(
        &ctx,
        serenity::EditMember::new().disable_communication_until(timestamp),
    )
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!(
                    "🔇 muted **{}** for {} min — {}",
                    name, minutes, reason
                ))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// unmute a member (remove timeout)
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
pub async fn unmute(
    ctx: Context<'_>,
    #[description = "user to unmute"] mut user: serenity::Member,
) -> Result<(), Error> {
    let name = user.user.name.clone();

    user.edit(
        &ctx,
        serenity::EditMember::new().disable_communication_until(String::new()),
    )
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("🔊 unmuted **{}**", name))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// warn a member (stored in database)
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
pub async fn warn(
    ctx: Context<'_>,
    #[description = "user to warn"] user: serenity::Member,
    #[description = "reason for the warning"] reason: String,
) -> Result<(), Error> {
    let name = user.user.name.clone();
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get();
    let moderator_id = ctx.author().id.get();
    let user_id = user.user.id.get();
    let guild_name = ctx
        .guild()
        .map(|g| g.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    sqlx::query("INSERT INTO warnings (guild_id, user_id, moderator_id, reason) VALUES ($1, $2, $3, $4)")
        .bind(guild_id as i64)
        .bind(user_id as i64)
        .bind(moderator_id as i64)
        .bind(&reason)
        .execute(&ctx.data().db)
        .await?;

    let warning_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM warnings WHERE guild_id = $1 AND user_id = $2")
        .bind(guild_id as i64)
        .bind(user_id as i64)
        .fetch_one(&ctx.data().db)
        .await?;

    let dm = user
        .user
        .dm(
            &ctx,
            serenity::CreateMessage::new()
                .content(format!("you have been warned in **{}** for: {} (total warnings: {})", guild_name, reason, warning_count)),
        )
        .await;

    if let Err(_e) = dm {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!(
                        "⚠️ warned **{}** — {} (could not dm) | total: {}",
                        name, reason, warning_count
                    ))
                    .color(0xF28080),
            ),
        )
        .await?;
    } else {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!(
                        "⚠️ warned **{}** — {} | total: {}",
                        name, reason, warning_count
                    ))
                    .color(0xF28080),
            ),
        )
        .await?;
    }

    Ok(())
}

/// view warnings for a user
#[poise::command(slash_command, required_permissions = "MODERATE_MEMBERS")]
pub async fn warnings(
    ctx: Context<'_>,
    #[description = "user to check"] user: Option<serenity::Member>,
) -> Result<(), Error> {
    let target = match user {
        Some(m) => m,
        None => ctx.author_member().await.ok_or("must be used in a guild")?.into_owned(),
    };

    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get();
    let user_id = target.user.id.get();

    let rows = sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, moderator_id, reason, created_at FROM warnings WHERE guild_id = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT 25",
    )
    .bind(guild_id as i64)
    .bind(user_id as i64)
    .fetch_all(&ctx.data().db)
    .await?;

    if rows.is_empty() {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!("**{}** has no warnings.", target.user.name))
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let total = rows.len();
    let mut description = String::new();
    for (id, moderator_id, reason, created_at) in &rows {
        let timestamp = created_at.format("%m/%d/%y %H:%M");
        description.push_str(&format!(
            "`#{}` — {} — {} — <@{}>\n",
            id, timestamp, reason, moderator_id
        ));
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(format!("{}'s warnings ({})", target.user.name, total))
                .description(&description)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// unban a member using their user id
#[poise::command(slash_command, required_permissions = "BAN_MEMBERS")]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "user id to unban"] user_id: u64,
) -> Result<(), Error> {
    let user = serenity::UserId::new(user_id);
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;

    match guild_id.unban(&ctx, user).await {
        Ok(_) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description(format!("🔓 unbanned <@!{}>", user_id))
                        .color(0xF28080),
                ),
            )
            .await?;
        }
        Err(e) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description(format!("could not unban that user: {}", e))
                        .color(0xF28080),
                ),
            )
            .await?;
        }
    }

    Ok(())
}

/// deletes messages from a channel (5-100)
#[poise::command(slash_command, required_permissions = "MANAGE_MESSAGES")]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "number of messages to delete (5-100)"] amount: Option<u64>,
) -> Result<(), Error> {
    let amount = amount.unwrap_or(5);

    if amount < 5 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("the amount of messages to delete must be at least **5**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if amount > 100 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("you can only delete **100** messages at a time!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let channel_id = ctx.channel_id();
    let messages = channel_id
        .messages(&ctx, serenity::GetMessages::new().limit(amount as u8))
        .await?;

    let msg_ids: Vec<serenity::MessageId> = messages.iter().map(|m| m.id).collect();
    channel_id.delete_messages(&ctx, &msg_ids).await?;

    let deleted = msg_ids.len();

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("deleted **{}** messages.", deleted))
                .color(0xF28080),
        ),
    )
    .await?;

    Ok(())
}

/// set the welcome message for this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn setwelcome(
    ctx: Context<'_>,
    #[description = "channel to send welcome messages"] channel: serenity::Channel,
    #[description = "message (use [mention], [server], [user], [name] as placeholders)"] message: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get();
    let channel_id = channel.id().get();

    sqlx::query(
        "INSERT INTO welcome (guild_id, channel_id, message) VALUES ($1, $2, $3) ON CONFLICT (guild_id) DO UPDATE SET channel_id = $2, message = $3",
    )
    .bind(guild_id as i64)
    .bind(channel_id as i64)
    .bind(&message)
    .execute(&ctx.data().db)
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ welcome message set to {}", channel.mention()))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// set the leave message for this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn setleave(
    ctx: Context<'_>,
    #[description = "channel to send leave messages"] channel: serenity::Channel,
    #[description = "message (use [mention], [server], [user], [name] as placeholders)"] message: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get();
    let channel_id = channel.id().get();

    sqlx::query(
        "INSERT INTO leave (guild_id, channel_id, message) VALUES ($1, $2, $3) ON CONFLICT (guild_id) DO UPDATE SET channel_id = $2, message = $3",
    )
    .bind(guild_id as i64)
    .bind(channel_id as i64)
    .bind(&message)
    .execute(&ctx.data().db)
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ leave message set to {}", channel.mention()))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

fn random_honeypot_name(existing: &[String]) -> String {
    let names = [
        "general", "chat", "lounge", "hangout", "off-topic",
        "watercooler", "random", "talk", "social", "chill",
        "discussion", "main", "lobby", "commons", "recreation",
        "hangout-2", "chat-2", "lounge-2", "vibes", "corner",
        "hangout-3", "hang", "yapping", "the-spot", "place",
        "room", "area", "zone", "hub", "space",
    ];
    let available: Vec<&str> = names.iter().copied().filter(|n| !existing.iter().any(|e| e == n)).collect();
    if let Some(&name) = available.choose(&mut rand::thread_rng()) {
        name.to_string()
    } else {
        format!("room-{}", rand::random::<u16>())
    }
}

/// manage honeypot channels (auto-bans anyone who chats in them)
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD", subcommands("create", "remove", "toggle"))]
pub async fn honeypot(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("use a subcommand: `create`, `remove`, or `toggle`").await?;
    Ok(())
}

/// create a honeypot channel that bans anyone who chats in it
#[poise::command(slash_command)]
pub async fn create(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let db = &ctx.data().db;
    let gid = guild_id.get() as i64;

    let existing = sqlx::query_scalar::<_, i64>("SELECT channel_id FROM honeypots WHERE guild_id = $1")
        .bind(gid)
        .fetch_optional(db)
        .await?;

    if existing.is_some() {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("this server already has a honeypot channel! use `/honeypot remove` first.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let channels = guild_id.channels(&ctx).await.unwrap_or_default();
    let existing_names: Vec<String> = channels.values().map(|c| c.name.clone()).collect();
    let name = random_honeypot_name(&existing_names);
    let channel = guild_id
        .create_channel(
            &ctx,
            serenity::CreateChannel::new(name).kind(serenity::ChannelType::Text),
        )
        .await?;

    let _ = channel.id.send_message(&ctx, serenity::CreateMessage::new()
        .embed(
            serenity::CreateEmbed::new()
                .description("this channel is monitored. if you are a regular user, **do not type here** or you will be banned automatically.")
                .color(0xF28080),
        )).await;

    sqlx::query("INSERT INTO honeypots (guild_id, channel_id) VALUES ($1, $2)")
        .bind(gid)
        .bind(channel.id.get() as i64)
        .execute(db)
        .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!(
                    "🪤 honeypot created: {} — anyone who chats here will be banned instantly.",
                    channel.mention()
                ))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// remove the honeypot channel
#[poise::command(slash_command)]
pub async fn remove(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let db = &ctx.data().db;
    let gid = guild_id.get() as i64;

    let channel_id = sqlx::query_scalar::<_, i64>("SELECT channel_id FROM honeypots WHERE guild_id = $1")
        .bind(gid)
        .fetch_optional(db)
        .await?;

    let channel_id = match channel_id {
        Some(id) => id,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("no honeypot channel exists in this server.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    sqlx::query("DELETE FROM honeypots WHERE guild_id = $1")
        .bind(gid)
        .execute(db)
        .await?;

    let _ = serenity::ChannelId::new(channel_id as u64).delete(&ctx).await;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description("✅ honeypot channel removed.")
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// toggle daily rotation (renames the honeypot to a random name each day)
#[poise::command(slash_command)]
pub async fn toggle(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let db = &ctx.data().db;
    let gid = guild_id.get() as i64;

    let row = sqlx::query_as::<_, (i64, bool)>("SELECT channel_id, rotate_daily FROM honeypots WHERE guild_id = $1")
        .bind(gid)
        .fetch_optional(db)
        .await?;

    let (channel_id, current) = match row {
        Some(r) => r,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("no honeypot channel exists in this server.")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let new_val = !current;
    sqlx::query("UPDATE honeypots SET rotate_daily = $1 WHERE guild_id = $2")
        .bind(new_val)
        .bind(gid)
        .execute(db)
        .await?;

    let status = if new_val { "enabled" } else { "disabled" };
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!(
                    "✅ daily rotation {} for <#{}>.",
                    status, channel_id
                ))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// check if a channel is a honeypot and ban the author if so
pub async fn handle_honeypot_message(
    http: &serenity::Http,
    db: &sqlx::PgPool,
    msg: &serenity::Message,
) {
    if msg.author.bot {
        return;
    }

    let channel_id = msg.channel_id.get() as i64;

    let row = sqlx::query_as::<_, (i64, bool)>("SELECT guild_id, rotate_daily FROM honeypots WHERE channel_id = $1")
        .bind(channel_id)
        .fetch_optional(db)
        .await;

    let (guild_id, _) = match row {
        Ok(Some(r)) => r,
        _ => return,
    };

    let guild = match serenity::GuildId::new(guild_id as u64).to_partial_guild(http).await {
        Ok(g) => g,
        Err(_) => return,
    };

    let reason = "sent a message in a honeypot channel";

    if let Err(e) = guild
        .ban_with_reason(http, msg.author.id, 0, reason)
        .await
    {
        tracing::error!("failed to ban honeypot user {}: {}", msg.author.id, e);
    }

    let _ = msg.delete(http).await;
}

/// rotate all honeypot channels that have daily rotation enabled
pub async fn rotate_honeypots(http: &serenity::Http, db: &sqlx::PgPool) {
    let rows = sqlx::query_as::<_, (i64, i64, Option<chrono::NaiveDate>)>(
        "SELECT guild_id, channel_id, last_rotated FROM honeypots WHERE rotate_daily = true",
    )
    .fetch_all(db)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return,
    };

    let today = chrono::Utc::now().date_naive();

    for (guild_id, channel_id, last_rotated) in rows {
        if last_rotated == Some(today) {
            continue;
        }

        let guild = serenity::GuildId::new(guild_id as u64);
        let existing_names: Vec<String> = match guild.channels(http).await {
            Ok(channels) => channels.values().map(|c| c.name.clone()).collect(),
            Err(_) => vec![],
        };
        let new_name = random_honeypot_name(&existing_names);
        let channel = serenity::ChannelId::new(channel_id as u64);

        if channel
            .edit(http, serenity::EditChannel::new().name(new_name))
            .await
            .is_ok()
        {
            let _ = channel.send_message(http, serenity::CreateMessage::new()
                .embed(
                    serenity::CreateEmbed::new()
                        .description("this channel has been renamed. if you are a regular user, **do not type here** or you will be banned automatically.")
                        .color(0xF28080),
                )).await;
            let _ = sqlx::query("UPDATE honeypots SET last_rotated = $1 WHERE guild_id = $2")
                .bind(today)
                .bind(guild_id)
                .execute(db)
                .await;
        }
    }
}
