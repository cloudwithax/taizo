use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

#[derive(Clone)]
struct SetupState {
    mode: String,
    title: String,
    description: String,
    max_roles: Option<i32>,
    role_duration: Option<i32>,
    pairs: Vec<(String, String)>,
    setup_msg_id: Option<u64>,
    channel_id: u64,
    started_at: std::time::Instant,
}

lazy_static::lazy_static! {
    static ref SETUP_STATE: Arc<RwLock<HashMap<u64, SetupState>>> =
        Arc::new(RwLock::new(HashMap::new()));
    static ref TEMP_TASKS: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>> =
        Arc::new(RwLock::new(Vec::new()));
}

fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim().to_lowercase();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
        let now = chrono::Utc::now();
        let diff = dt.signed_duration_since(now);
        if diff.num_seconds() > 0 {
            return Some(Duration::from_secs(diff.num_seconds() as u64));
        }
        return None;
    }
    let num: u64 = s
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()?;
    let unit: String = s.chars().skip_while(|c| c.is_ascii_digit()).collect();
    match unit.as_str() {
        "m" | "min" | "mins" | "minute" | "minutes" => Some(Duration::from_secs(num * 60)),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(Duration::from_secs(num * 3600)),
        "d" | "day" | "days" => Some(Duration::from_secs(num * 86400)),
        "w" | "wk" | "wks" | "week" | "weeks" => Some(Duration::from_secs(num * 604800)),
        _ => None,
    }
}

fn parse_reaction_type(s: &str) -> Option<serenity::ReactionType> {
    let s = s.trim();
    if s.starts_with('<') && s.ends_with('>') {
        let inner = &s[1..s.len() - 1];
        if inner.starts_with("a:") {
            let rest = &inner[2..];
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

fn build_reaction_role_embed(
    title: &str,
    description: &str,
    pairs: &[(String, String)],
    mode: &str,
) -> serenity::CreateEmbed {
    let mut pair_lines = String::new();
    for (emoji, role_id) in pairs {
        pair_lines.push_str(&format!("{} <@&{}>\n", emoji, role_id));
    }
    if pair_lines.is_empty() {
        pair_lines = "no pairs yet".to_string();
    }

    let mut embed = serenity::CreateEmbed::new()
        .title(title)
        .color(0x80F291);

    if !description.is_empty() {
        embed = embed.description(format!("{}\n\n{}", description, pair_lines));
    } else {
        embed = embed.description(pair_lines);
    }

    embed = embed.field("mode", mode, true);
    embed = embed.footer(serenity::CreateEmbedFooter::new(
        "react with an emoji to get the role",
    ));

    embed
}

fn build_setup_embed(state: &SetupState) -> serenity::CreateEmbed {
    let mut pair_lines = String::new();
    for (i, (emoji, role_id)) in state.pairs.iter().enumerate() {
        pair_lines.push_str(&format!("{}. {} <@&{}>\n", i + 1, emoji, role_id));
    }
    if pair_lines.is_empty() {
        pair_lines = "no pairs added yet".to_string();
    }

    let mut embed = serenity::CreateEmbed::new()
        .title("reaction role setup")
        .description(pair_lines)
        .color(0xF2D380);

    embed = embed.field("mode", &state.mode, true);
    if !state.title.is_empty() {
        embed = embed.field("title", &state.title, true);
    }
    if !state.description.is_empty() {
        embed = embed.field("description", &state.description, true);
    }
    if let Some(max) = state.max_roles {
        embed = embed.field("max roles", &max.to_string(), true);
    }
    if let Some(dur) = state.role_duration {
        embed = embed.field("duration", &format!("{}s", dur), true);
    }
    embed = embed.footer(serenity::CreateEmbedFooter::new(format!(
        "{} pairs added",
        state.pairs.len()
    )));

    embed
}

/// manage reaction roles (create, remove, list)
#[poise::command(
    slash_command,
    required_permissions = "MANAGE_ROLES",
    subcommands("rr_create", "rr_remove", "rr_list")
)]
pub async fn reactionrole(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("use a subcommand: `create`, `remove`, or `list`")
        .await?;
    Ok(())
}

/// create a reaction role message with interactive setup
#[poise::command(slash_command)]
pub async fn rr_create(
    ctx: Context<'_>,
    #[description = "mode: normal, reverse, unique, permanent, limit, verify, temp"] mode: String,
    #[description = "title for the embed"] title: Option<String>,
    #[description = "description for the embed"] description: Option<String>,
    #[description = "max roles per user (for limit mode)"] max: Option<i32>,
    #[description = "duration for temp mode (e.g. 30m, 1h, 2d)"] duration: Option<String>,
) -> Result<(), Error> {
    let mode_lower = mode.to_lowercase();
    if !["normal", "reverse", "unique", "permanent", "limit", "verify", "temp"]
        .contains(&mode_lower.as_str())
    {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("invalid mode. use: normal, reverse, unique, permanent, limit, verify, or temp")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let role_duration = if mode_lower == "temp" {
        match duration {
            Some(d) => match parse_duration(&d) {
                Some(dur) => Some(dur.as_secs() as i32),
                None => {
                    ctx.send(
                        poise::CreateReply::default().embed(
                            serenity::CreateEmbed::new()
                                .description("invalid duration format. use e.g. `30m`, `1h`, `2d`, `1w` or an iso timestamp")
                                .color(0xF28080),
                        ),
                    )
                    .await?;
                    return Ok(());
                }
            },
            None => {
                ctx.send(
                    poise::CreateReply::default().embed(
                        serenity::CreateEmbed::new()
                            .description("temp mode requires a `duration` parameter (e.g. `30m`, `1h`, `2d`)")
                            .color(0xF28080),
                    ),
                )
                .await?;
                return Ok(());
            }
        }
    } else if mode_lower == "limit" && max.is_none() {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("limit mode requires a `max` parameter")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    } else {
        None
    };

    let user_id = ctx.author().id.get();
    let channel_id = ctx.channel_id().get();
    let state = SetupState {
        mode: mode_lower,
        title: title.unwrap_or_else(|| "reaction roles".to_string()),
        description: description.unwrap_or_default(),
        max_roles: max,
        role_duration,
        pairs: Vec::new(),
        setup_msg_id: None,
        channel_id,
        started_at: std::time::Instant::now(),
    };

    {
        let mut states = SETUP_STATE.write().await;
        states.insert(user_id, state);
    }

    let embed = {
        let states = SETUP_STATE.read().await;
        let state = states.get(&user_id).unwrap();
        build_setup_embed(state)
    };

    let add_button = serenity::CreateButton::new("rr_add_pairs")
        .label("add pairs")
        .style(serenity::ButtonStyle::Success);

    let finish_button = serenity::CreateButton::new("rr_finish")
        .label("finish")
        .style(serenity::ButtonStyle::Primary);

    let cancel_button = serenity::CreateButton::new("rr_cancel")
        .label("cancel")
        .style(serenity::ButtonStyle::Danger);

    let row = serenity::CreateActionRow::Buttons(vec![add_button, finish_button, cancel_button]);

    let sent = ctx
        .send(
            poise::CreateReply::default()
                .embed(embed)
                .components(vec![row])
                .ephemeral(true),
        )
        .await?;
    let msg = sent.message().await?;

    {
        let mut states = SETUP_STATE.write().await;
        if let Some(state) = states.get_mut(&user_id) {
            state.setup_msg_id = Some(msg.id.get());
        }
    }

    Ok(())
}

/// remove a reaction role message
#[poise::command(slash_command)]
pub async fn rr_remove(
    ctx: Context<'_>,
    #[description = "message id or link"] message: String,
) -> Result<(), Error> {
    let message_id = if message.contains("channels") {
        message
            .split('/')
            .last()
            .and_then(|s| s.parse::<u64>().ok())
    } else {
        message.parse::<u64>().ok()
    };

    let message_id = match message_id {
        Some(id) => id,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("invalid message id or link")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let db = &ctx.data().db;
    let msg_id_i64 = message_id as i64;

    let row = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT guild_id, channel_id, mode FROM reaction_roles WHERE message_id = $1",
    )
    .bind(msg_id_i64)
    .fetch_optional(db)
    .await?;

    let (guild_id, channel_id, mode) = match row {
        Some(r) => r,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("no reaction role message found with that id")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    if guild_id != ctx.guild_id().ok_or("must be used in a guild")?.get() as i64 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("that reaction role message is not in this server")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let assignments = sqlx::query_as::<_, (i64, i64)>(
        "SELECT user_id, role_id FROM reaction_role_users WHERE message_id = $1",
    )
    .bind(msg_id_i64)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    if mode != "permanent" && mode != "verify" {
        let http = ctx.http();
        for (uid, rid) in &assignments {
            let user = serenity::UserId::new(*uid as u64);
            let guild = serenity::GuildId::new(guild_id as u64);
            if let Ok(member) = guild.member(http, user).await {
                let _ = member
                    .remove_role(http, serenity::RoleId::new(*rid as u64))
                    .await;
            }
        }
    }

    sqlx::query("DELETE FROM reaction_roles WHERE message_id = $1")
        .bind(msg_id_i64)
        .execute(db)
        .await?;

    let ch = serenity::ChannelId::new(channel_id as u64);
    let _ = ch
        .delete_message(ctx.http(), serenity::MessageId::new(message_id))
        .await;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description("reaction role message removed")
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// list all reaction role messages in this server
#[poise::command(slash_command)]
pub async fn rr_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get() as i64;
    let db = &ctx.data().db;

    let rows = sqlx::query_as::<_, (i64, i64, String, String, Option<String>, Option<i32>)>(
        "SELECT rr.message_id, rr.channel_id, rr.title, rr.mode, rr.description, rr.max_roles FROM reaction_roles rr WHERE rr.guild_id = $1 ORDER BY rr.message_id DESC",
    )
    .bind(guild_id)
    .fetch_all(db)
    .await?;

    if rows.is_empty() {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("no reaction role messages in this server")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let pair_counts = sqlx::query_as::<_, (i64, i64)>(
        "SELECT message_id, COUNT(*) FROM reaction_role_pairs GROUP BY message_id",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let pair_map: HashMap<i64, i64> = pair_counts.into_iter().collect();

    let mut description = String::new();
    for (msg_id, ch_id, title, mode, desc, max_roles) in &rows {
        let pair_count = pair_map.get(msg_id).copied().unwrap_or(0);
        let mut line = format!(
            "[{}] <#{}> — **{}** — mode: `{}` — {} pair{}",
            msg_id,
            ch_id,
            title,
            mode,
            pair_count,
            if pair_count == 1 { "" } else { "s" }
        );
        if let Some(max) = max_roles {
            line.push_str(&format!(" — max: {}", max));
        }
        if let Some(d) = desc {
            if !d.is_empty() {
                line.push_str(&format!(" — {}", d));
            }
        }
        description.push_str(&line);
        description.push('\n');
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(format!("reaction roles ({})", rows.len()))
                .description(description)
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

pub async fn handle_setup_button(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    db: &sqlx::PgPool,
) {
    let user_id = interaction.user.id.get();

    if interaction.data.custom_id == "rr_cancel" {
        let mut states = SETUP_STATE.write().await;
        states.remove(&user_id);
        let _ = interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("setup cancelled")
                        .embeds(Vec::new())
                        .components(Vec::new()),
                ),
            )
            .await;
        return;
    }

    if interaction.data.custom_id == "rr_add_pairs" {
        {
            let states = SETUP_STATE.read().await;
            if !states.contains_key(&user_id) {
                let _ = interaction
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new()
                                .content("setup expired or cancelled. run `/reactionrole create` again")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
        }

        let rows = vec![
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "pair 1 (emoji role_id)",
                    "pair1",
                )
                .placeholder("🍎 123456789012345678")
                .required(false),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "pair 2 (emoji role_id)",
                    "pair2",
                )
                .placeholder("🔥 987654321098765432")
                .required(false),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "pair 3 (emoji role_id)",
                    "pair3",
                )
                .placeholder("🟢 111222333444555666")
                .required(false),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "pair 4 (emoji role_id)",
                    "pair4",
                )
                .placeholder("🔵 666555444333222111")
                .required(false),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "pair 5 (emoji role_id)",
                    "pair5",
                )
                .placeholder("🟡 999888777666555444")
                .required(false),
            ),
        ];

        let modal = serenity::CreateInteractionResponse::Modal(
            serenity::CreateModal::new(format!("rr_modal_{}", user_id), "add reaction role pairs")
                .components(rows),
        );

        let _ = interaction.create_response(ctx, modal).await;
        return;
    }

    if interaction.data.custom_id == "rr_finish" {
        let state = {
            let states = SETUP_STATE.read().await;
            states.get(&user_id).cloned()
        };

        let state = match state {
            Some(s) => s,
            None => {
                let _ = interaction
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new()
                                .content("setup expired or cancelled. run `/reactionrole create` again")
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
        };

        if state.pairs.is_empty() {
            let _ = interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("add at least one pair first!")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        if state.pairs.len() > 20 {
            let _ = interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("maximum 20 pairs allowed!")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        let guild_id = serenity::GuildId::new(
            interaction
                .guild_id
                .map(|g| g.get())
                .unwrap_or(0),
        );

        let mut valid_pairs = Vec::new();
        for (emoji_str, role_id_str) in &state.pairs {
            let role_id: u64 = match role_id_str.parse() {
                Ok(id) => id,
                Err(_) => continue,
            };

            let role = match guild_id.role(&*ctx.http, serenity::RoleId::new(role_id)).await {
                Ok(r) => r,
                Err(_) => continue,
            };

            let bot_user_id = ctx.cache.current_user().id;
            let bot_member =
                match guild_id.member(&*ctx.http, bot_user_id).await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

            let bot_highest_pos = if let Some(guild) = ctx.cache.guild(guild_id) {
                bot_member.roles.iter()
                    .filter_map(|r| guild.roles.get(r).map(|role| role.position))
                    .max()
                    .unwrap_or(0)
            } else {
                0
            };
            if bot_highest_pos <= role.position {
                continue;
            }

            if role.name == "@everyone" {
                continue;
            }

            valid_pairs.push((emoji_str.clone(), role_id));
        }

        if valid_pairs.is_empty() {
            let _ = interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("no valid pairs found. make sure role ids are correct and the bot's role is above them")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }

        let embed = build_reaction_role_embed(
            &state.title,
            &state.description,
            &valid_pairs
                .iter()
                .map(|(e, r)| (e.clone(), r.to_string()))
                .collect::<Vec<_>>(),
            &state.mode,
        );

        let channel = serenity::ChannelId::new(state.channel_id);
        let msg = match channel
            .send_message(&*ctx.http, serenity::CreateMessage::new().embed(embed))
            .await
        {
            Ok(m) => m,
            Err(e) => {
                let _ = interaction
                    .create_response(
                        ctx,
                        serenity::CreateInteractionResponse::Message(
                            serenity::CreateInteractionResponseMessage::new()
                                .content(format!("failed to send message: {}", e))
                                .ephemeral(true),
                        ),
                    )
                    .await;
                return;
            }
        };

        for (emoji_str, _) in &valid_pairs {
            if let Some(reaction_type) = parse_reaction_type(emoji_str) {
                let _ = msg.react(&*ctx.http, reaction_type).await;
            }
        }

        let msg_id = msg.id.get() as i64;
        let guild_id_val = guild_id.get() as i64;
        let channel_id_val = state.channel_id as i64;
        let created_by = user_id as i64;

        let _ = sqlx::query(
            "INSERT INTO reaction_roles (message_id, guild_id, channel_id, mode, max_roles, role_duration, created_by, title, description) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(msg_id)
        .bind(guild_id_val)
        .bind(channel_id_val)
        .bind(&state.mode)
        .bind(state.max_roles)
        .bind(state.role_duration)
        .bind(created_by)
        .bind(&state.title)
        .bind(&state.description)
        .execute(db)
        .await;

        for (emoji_str, role_id) in &valid_pairs {
            let _ = sqlx::query(
                "INSERT INTO reaction_role_pairs (message_id, emoji, role_id) VALUES ($1, $2, $3)",
            )
            .bind(msg_id)
            .bind(emoji_str)
            .bind(*role_id as i64)
            .execute(db)
            .await;
        }

        {
            let mut states = SETUP_STATE.write().await;
            states.remove(&user_id);
        }

        let msg_link = format!(
            "https://discord.com/channels/{}/{}/{}",
            guild_id.get(),
            state.channel_id,
            msg.id.get()
        );

        let _ = interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!("reaction role message created: {}", msg_link))
                        .embeds(Vec::new())
                        .components(Vec::new()),
                ),
            )
            .await;
        return;
    }
}

pub async fn handle_modal_submit(
    ctx: &serenity::Context,
    interaction: &serenity::ModalInteraction,
    _db: &sqlx::PgPool,
) {
    let user_id = interaction.user.id.get();

    let mut states = SETUP_STATE.write().await;
    let state = match states.get_mut(&user_id) {
        Some(s) => s,
        None => {
            let _ = interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("setup expired or cancelled. run `/reactionrole create` again")
                            .ephemeral(true),
                    ),
                )
                .await;
            return;
        }
    };

    if state.started_at.elapsed() > Duration::from_secs(300) {
        states.remove(&user_id);
        let _ = interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("setup timed out. run `/reactionrole create` again")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let mut new_pairs = Vec::new();
    for action_row in &interaction.data.components {
        for component in &action_row.components {
            if let serenity::ActionRowComponent::InputText(text_input) = component {
                if let Some(ref value) = text_input.value {
                    let trimmed = value.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let emoji = parts[0].to_string();
                        let role_id = parts[1].to_string();
                        if role_id.parse::<u64>().is_ok() {
                            new_pairs.push((emoji, role_id));
                        }
                    }
                }
            }
        }
    }

    if new_pairs.is_empty() {
        let _ = interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content("no valid pairs found. use format: `emoji role_id`")
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let current_count = state.pairs.len();
    if current_count + new_pairs.len() > 20 {
        let _ = interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(format!(
                            "would exceed 20 pair limit. you have {} pairs, adding {} would be {}",
                            current_count,
                            new_pairs.len(),
                            current_count + new_pairs.len()
                        ))
                        .ephemeral(true),
                ),
            )
            .await;
        return;
    }

    let added = new_pairs.len();
    state.pairs.extend(new_pairs);

    let embed = build_setup_embed(state);
    let setup_msg_id = state.setup_msg_id;

    let _ = interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(format!(
                        "added {} pair{} ({} total)",
                        added,
                        if added == 1 { "" } else { "s" },
                        state.pairs.len()
                    ))
                    .ephemeral(true),
            ),
        )
        .await;

    if let Some(msg_id) = setup_msg_id {
        let channel = serenity::ChannelId::new(state.channel_id);
        let _ = channel
            .edit_message(
                &*ctx.http,
                serenity::MessageId::new(msg_id),
                serenity::EditMessage::new().embed(embed),
            )
            .await;
    }
}

pub async fn handle_reaction_add(
    ctx: &serenity::Context,
    reaction: &serenity::Reaction,
    db: &sqlx::PgPool,
) {
    let user_id = match reaction.user_id {
        Some(u) => u,
        None => return,
    };
    let msg_id = reaction.message_id.get() as i64;
    let user_id_i64 = user_id.get() as i64;
    let emoji_str = reaction_to_string(&reaction.emoji);
    let http = &*ctx.http;

    if user_id == ctx.cache.current_user().id {
        return;
    }

    let row = sqlx::query_as::<_, (String, Option<i32>, Option<i32>)>(
        "SELECT mode, max_roles, role_duration FROM reaction_roles WHERE message_id = $1",
    )
    .bind(msg_id)
    .fetch_optional(db)
    .await;

    let (mode, max_roles, role_duration) = match row {
        Ok(Some(r)) => r,
        _ => return,
    };

    let pair = sqlx::query_as::<_, (i64,)>(
        "SELECT role_id FROM reaction_role_pairs WHERE message_id = $1 AND emoji = $2",
    )
    .bind(msg_id)
    .bind(&emoji_str)
    .fetch_optional(db)
    .await;

    let role_id = match pair {
        Ok(Some((rid,))) => rid as u64,
        _ => return,
    };

    let guild_id = reaction
        .guild_id
        .or_else(|| {
            ctx.cache
                .message(reaction.channel_id, reaction.message_id)
                .and_then(|m| m.guild_id)
        });

    let guild_id = match guild_id {
        Some(g) => g,
        None => return,
    };

    let member = match guild_id.member(http, user_id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let role = serenity::RoleId::new(role_id);

    match mode.as_str() {
        "normal" => {
            let _ = member.add_role(http, role).await;
            let _ = sqlx::query(
                "INSERT INTO reaction_role_users (message_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .bind(role_id as i64)
            .execute(db)
            .await;
        }
        "reverse" => {
            if member.roles.contains(&role) {
                let _ = member.remove_role(http, role).await;
                let _ = sqlx::query("DELETE FROM reaction_role_users WHERE message_id = $1 AND user_id = $2 AND role_id = $3")
                    .bind(msg_id)
                    .bind(user_id_i64)
                    .bind(role_id as i64)
                    .execute(db)
                    .await;
            }
        }
        "unique" => {
            let existing = sqlx::query_as::<_, (i64,)>(
                "SELECT role_id FROM reaction_role_users WHERE message_id = $1 AND user_id = $2",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .fetch_all(db)
            .await
            .unwrap_or_default();

            for (old_role_id,) in existing {
                let _ = member
                    .remove_role(http, serenity::RoleId::new(old_role_id as u64))
                    .await;
            }
            let _ = sqlx::query(
                "DELETE FROM reaction_role_users WHERE message_id = $1 AND user_id = $2",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .execute(db)
            .await;

            let _ = member.add_role(http, role).await;
            let _ = sqlx::query(
                "INSERT INTO reaction_role_users (message_id, user_id, role_id) VALUES ($1, $2, $3)",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .bind(role_id as i64)
            .execute(db)
            .await;
        }
        "permanent" => {
            let _ = member.add_role(http, role).await;
            let _ = sqlx::query(
                "INSERT INTO reaction_role_users (message_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .bind(role_id as i64)
            .execute(db)
            .await;
        }
        "limit" => {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM reaction_role_users WHERE message_id = $1 AND user_id = $2",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .fetch_one(db)
            .await
            .unwrap_or(0);

            let max = max_roles.unwrap_or(1);
            if count < max as i64 {
                let _ = member.add_role(http, role).await;
                let _ = sqlx::query(
                    "INSERT INTO reaction_role_users (message_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(msg_id)
                .bind(user_id_i64)
                .bind(role_id as i64)
                .execute(db)
                .await;
            } else {
                let _ = http
                    .delete_reaction(
                        reaction.channel_id,
                        reaction.message_id,
                        user_id,
                        &reaction.emoji,
                    )
                    .await;
            }
        }
        "verify" => {
            let _ = member.add_role(http, role).await;
            let _ = sqlx::query(
                "INSERT INTO reaction_role_users (message_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .bind(role_id as i64)
            .execute(db)
            .await;

            let _ = http
                .delete_reaction(
                    reaction.channel_id,
                    reaction.message_id,
                    user_id,
                    &reaction.emoji,
                )
                .await;
        }
        "temp" => {
            let _ = member.add_role(http, role).await;

            let expires_at = chrono::Utc::now()
                + chrono::Duration::seconds(role_duration.unwrap_or(3600) as i64);

            let _ = sqlx::query(
                "INSERT INTO reaction_role_users (message_id, user_id, role_id, expires_at) VALUES ($1, $2, $3, $4) ON CONFLICT (message_id, user_id, role_id) DO UPDATE SET expires_at = $4",
            )
            .bind(msg_id)
            .bind(user_id_i64)
            .bind(role_id as i64)
            .bind(expires_at)
            .execute(db)
            .await;

            let http = ctx.http.clone();
            let db_clone = db.clone();
            let gid = guild_id;
            let rid = role;
            let delay = (expires_at - chrono::Utc::now())
                .to_std()
                .unwrap_or(Duration::from_secs(60));

            let handle = tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                if let Ok(member) = gid.member(&*http, user_id).await {
                    let _ = member.remove_role(&*http, rid).await;
                }
                let _ = sqlx::query(
                    "DELETE FROM reaction_role_users WHERE message_id = $1 AND user_id = $2 AND role_id = $3",
                )
                .bind(msg_id)
                .bind(user_id_i64)
                .bind(rid.get() as i64)
                .execute(&db_clone)
                .await;
            });

            let mut tasks = TEMP_TASKS.write().await;
            tasks.push(handle);
        }
        _ => {}
    }
}

pub async fn handle_reaction_remove(
    ctx: &serenity::Context,
    reaction: &serenity::Reaction,
    db: &sqlx::PgPool,
) {
    let user_id = match reaction.user_id {
        Some(u) => u,
        None => return,
    };
    let msg_id = reaction.message_id.get() as i64;
    let user_id_i64 = user_id.get() as i64;
    let emoji_str = reaction_to_string(&reaction.emoji);
    let http = &*ctx.http;

    if user_id == ctx.cache.current_user().id {
        return;
    }

    let row = sqlx::query_as::<_, (String,)>(
        "SELECT mode FROM reaction_roles WHERE message_id = $1",
    )
    .bind(msg_id)
    .fetch_optional(db)
    .await;

    let mode = match row {
        Ok(Some((m,))) => m,
        _ => return,
    };

    let pair = sqlx::query_as::<_, (i64,)>(
        "SELECT role_id FROM reaction_role_pairs WHERE message_id = $1 AND emoji = $2",
    )
    .bind(msg_id)
    .bind(&emoji_str)
    .fetch_optional(db)
    .await;

    let role_id = match pair {
        Ok(Some((rid,))) => rid as u64,
        _ => return,
    };

    let guild_id = reaction
        .guild_id
        .or_else(|| {
            ctx.cache
                .message(reaction.channel_id, reaction.message_id)
                .and_then(|m| m.guild_id)
        });

    let guild_id = match guild_id {
        Some(g) => g,
        None => return,
    };

    let member = match guild_id.member(http, user_id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    let role = serenity::RoleId::new(role_id);

    match mode.as_str() {
        "normal" | "unique" | "limit" => {
            let _ = member.remove_role(http, role).await;
            let _ = sqlx::query("DELETE FROM reaction_role_users WHERE message_id = $1 AND user_id = $2 AND role_id = $3")
                .bind(msg_id)
                .bind(user_id_i64)
                .bind(role_id as i64)
                .execute(db)
                .await;
        }
        "reverse" => {
            if !member.roles.contains(&role) {
                let _ = member.add_role(http, role).await;
                let _ = sqlx::query(
                    "INSERT INTO reaction_role_users (message_id, user_id, role_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                )
                .bind(msg_id)
                .bind(user_id_i64)
                .bind(role_id as i64)
                .execute(db)
                .await;
            }
        }
        "permanent" | "verify" | "temp" => {}
        _ => {}
    }
}
