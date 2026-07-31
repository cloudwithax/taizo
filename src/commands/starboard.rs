use crate::{Context, Error};
use poise::serenity_prelude as serenity;

fn reaction_to_string(emoji: &serenity::ReactionType) -> String {
    match emoji {
        serenity::ReactionType::Unicode(s) => s.clone(),
        serenity::ReactionType::Custom {
            animated,
            id,
            name,
        } => {
            let prefix = if *animated { "a:" } else { "" };
            let n = name.as_deref().unwrap_or("");
            format!("{}{}:{}", prefix, n, id)
        }
        _ => "unknown".to_string(),
    }
}

fn parse_reaction_type(s: &str) -> Option<serenity::ReactionType> {
    let s = s.trim();
    if s.starts_with('<') && s.ends_with('>') {
        let inner = &s[1..s.len() - 1];
        if let Some(rest) = inner.strip_prefix("a:") {
            if let Some((name, id_str)) = rest.rsplit_once(':') {
                if let Ok(id) = id_str.parse::<u64>() {
                    return Some(serenity::ReactionType::Custom {
                        animated: true,
                        id: serenity::EmojiId::new(id),
                        name: Some(name.to_string()),
                    });
                }
            }
        } else if let Some((name, id_str)) = inner.rsplit_once(':') {
            if let Ok(id) = id_str.parse::<u64>() {
                return Some(serenity::ReactionType::Custom {
                    animated: false,
                    id: serenity::EmojiId::new(id),
                    name: Some(name.to_string()),
                });
            }
        }
    }
    if !s.is_empty() {
        return Some(serenity::ReactionType::Unicode(s.to_string()));
    }
    None
}

fn build_starboard_embed(
    msg: &serenity::Message,
    star_count: i32,
    channel_name: &str,
    reply_snippet: Option<(&str, &str)>,
) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .color(0xFFD700)
        .timestamp(msg.timestamp);

    embed = embed.author(
        serenity::CreateEmbedAuthor::new(&msg.author.name)
            .icon_url(msg.author.face()),
    );

    if !msg.content.is_empty() {
        let content = if msg.content.len() > 2000 {
            format!("{}...", &msg.content[..2000])
        } else {
            msg.content.clone()
        };
        embed = embed.description(&content);
    }

    if let Some((author, preview)) = reply_snippet {
        embed = embed.field(
            format!("replying to {}", author),
            preview,
            false,
        );
    }

    for attachment in &msg.attachments {
        if attachment.height.is_some() && attachment.width.is_some() {
            embed = embed.image(&attachment.url);
            break;
        }
    }

    embed = embed.footer(
        serenity::CreateEmbedFooter::new(format!(
            "\u{2b50} {} \u{2022} #{}",
            star_count,
            channel_name,
        )),
    );

    let jump_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        msg.guild_id.map(|g| g.get()).unwrap_or(0),
        msg.channel_id.get(),
        msg.id.get(),
    );
    embed = embed.url(jump_url);

    embed
}

pub async fn handle_reaction(
    ctx: &serenity::Context,
    reaction: &serenity::Reaction,
    db: &sqlx::PgPool,
) {
    let user_id = reaction.user_id;
    if let Some(uid) = user_id {
        if uid == ctx.cache.current_user().id {
            return;
        }
    }

    let guild_id = match reaction.guild_id {
        Some(g) => g,
        None => return,
    };

    let config = sqlx::query_as::<_, (i64, i32, String)>(
        "SELECT channel_id, threshold, emoji FROM starboard_config WHERE guild_id = $1",
    )
    .bind(guild_id.get() as i64)
    .fetch_optional(db)
    .await;

    let (starboard_channel_id, threshold, emoji_str) = match config {
        Ok(Some(c)) => c,
        _ => return,
    };

    let starboard_channel = serenity::ChannelId::new(starboard_channel_id as u64);

    if reaction.channel_id == starboard_channel {
        return;
    }

    let reaction_str = reaction_to_string(&reaction.emoji);
    if reaction_str != emoji_str {
        return;
    }

    let emoji_type = match parse_reaction_type(&emoji_str) {
        Some(e) => e,
        None => return,
    };

    let msg = match reaction.message(ctx).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let star_count = match ctx
        .http
        .get_reaction_users(
            reaction.channel_id,
            reaction.message_id,
            &emoji_type,
            100,
            None,
        )
        .await
    {
        Ok(users) => users.len() as i32,
        Err(_) => return,
    };

    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT starboard_msg_id FROM starboard_messages WHERE message_id = $1",
    )
    .bind(reaction.message_id.get() as i64)
    .fetch_optional(db)
    .await;

    let channel_name = ctx
        .cache
        .guild(guild_id)
        .and_then(|g| g.channels.get(&reaction.channel_id).cloned())
        .map(|c| c.name)
        .unwrap_or_else(|| "unknown".to_string());

    let reply_snippet = if let Some(ref reference) = msg.message_reference {
        if let Some(ref_id) = reference.message_id {
            if let Ok(ref_msg) = ctx.http.get_message(reaction.channel_id, ref_id).await {
                let preview = if ref_msg.content.is_empty() {
                    "*(no text)*".to_string()
                } else if ref_msg.content.len() > 100 {
                    format!("{}...", &ref_msg.content[..100])
                } else {
                    ref_msg.content.clone()
                };
                Some((ref_msg.author.name.clone(), preview))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if star_count >= threshold {
        let embed = build_starboard_embed(&msg, star_count, &channel_name, reply_snippet.as_ref().map(|(a, p)| (a.as_str(), p.as_str())));

        match existing {
            Ok(Some((msg_id,))) => {
                let _ = ctx
                    .http
                    .edit_message(
                        starboard_channel,
                        serenity::MessageId::new(msg_id as u64),
                        &serenity::EditMessage::new().embed(embed),
                        Vec::<serenity::CreateAttachment>::new(),
                    )
                    .await;

                let _ = sqlx::query("UPDATE starboard_messages SET star_count = $1 WHERE message_id = $2")
                    .bind(star_count)
                    .bind(reaction.message_id.get() as i64)
                    .execute(db)
                    .await;
            }
            _ => {
                if let Ok(sent) = starboard_channel
                    .send_message(&ctx.http, serenity::CreateMessage::new().embed(embed))
                    .await
                {
                    let _ = sqlx::query(
                        "INSERT INTO starboard_messages (message_id, guild_id, channel_id, starboard_msg_id, star_count) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (message_id) DO UPDATE SET starboard_msg_id = $4, star_count = $5",
                    )
                    .bind(reaction.message_id.get() as i64)
                    .bind(guild_id.get() as i64)
                    .bind(reaction.channel_id.get() as i64)
                    .bind(sent.id.get() as i64)
                    .bind(star_count)
                    .execute(db)
                    .await;
                }
            }
        }
    } else if let Ok(Some((msg_id,))) = existing {
        let _ = ctx
            .http
            .delete_message(
                starboard_channel,
                serenity::MessageId::new(msg_id as u64),
                Some("starboard threshold no longer met"),
            )
            .await;

        let _ = sqlx::query("DELETE FROM starboard_messages WHERE message_id = $1")
            .bind(reaction.message_id.get() as i64)
            .execute(db)
            .await;
    }
}

/// manage the starboard
#[poise::command(
    slash_command,
    category = "utility",
    required_permissions = "ADMINISTRATOR",
    subcommands("setup", "config", "setemoji")
)]
pub async fn starboard(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("use a subcommand: `setup`, `config`, or `setemoji`")
        .await?;
    Ok(())
}

/// set up the starboard channel and threshold
#[poise::command(slash_command)]
pub async fn setup(
    ctx: Context<'_>,
    #[description = "channel for the starboard"] channel: serenity::Channel,
    #[description = "star threshold (default 5)"] threshold: Option<i32>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;
    let cid = channel.id().get() as i64;
    let threshold = threshold.unwrap_or(5).max(1);

    sqlx::query(
        "INSERT INTO starboard_config (guild_id, channel_id, threshold, emoji) VALUES ($1, $2, $3, '\u{2b50}') ON CONFLICT (guild_id) DO UPDATE SET channel_id = $2, threshold = $3",
    )
    .bind(gid)
    .bind(cid)
    .bind(threshold)
    .execute(&ctx.data().db)
    .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("starboard configured")
                .field("channel", format!("<#{}>", cid), false)
                .field("threshold", threshold.to_string(), false)
                .field("emoji", "\u{2b50}", false)
                .color(0xFFD700),
        ),
    )
    .await?;
    Ok(())
}

/// view starboard settings
#[poise::command(slash_command)]
pub async fn config(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;

    let row = sqlx::query_as::<_, (i64, i32, String)>(
        "SELECT channel_id, threshold, emoji FROM starboard_config WHERE guild_id = $1",
    )
    .bind(gid)
    .fetch_optional(&ctx.data().db)
    .await?;

    match row {
        Some((cid, threshold, emoji)) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .title("starboard config")
                        .field("channel", format!("<#{}>", cid), false)
                        .field("threshold", threshold.to_string(), false)
                        .field("emoji", &emoji, false)
                        .color(0xFFD700),
                ),
            )
            .await?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("no starboard configured yet. use `/starboard setup` to set one up.")
                        .color(0xF28080),
                ),
            )
            .await?;
        }
    }
    Ok(())
}

/// set a custom star emoji for the starboard
#[poise::command(slash_command)]
pub async fn setemoji(
    ctx: Context<'_>,
    #[description = "emoji to use (unicode or custom like <:name:id>)"] emoji: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let gid = guild_id.get() as i64;

    let parsed = parse_reaction_type(&emoji);
    if parsed.is_none() {
        ctx.say("invalid emoji").await?;
        return Ok(());
    }

    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT guild_id FROM starboard_config WHERE guild_id = $1",
    )
    .bind(gid)
    .fetch_optional(&ctx.data().db)
    .await?;

    if exists.is_none() {
        ctx.say("set up the starboard first with `/starboard setup`")
            .await?;
        return Ok(());
    }

    sqlx::query("UPDATE starboard_config SET emoji = $1 WHERE guild_id = $2")
        .bind(&emoji)
        .bind(gid)
        .execute(&ctx.data().db)
        .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("starboard emoji updated")
                .field("new emoji", &emoji, false)
                .color(0xFFD700),
        ),
    )
    .await?;
    Ok(())
}
