use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;

lazy_static::lazy_static! {
    static ref SNIPE_CACHE: Arc<RwLock<std::collections::HashMap<u64, Vec<serenity::Message>>>> =
        Arc::new(RwLock::new(std::collections::HashMap::new()));
    static ref DELETED_MSG_CACHE: Arc<RwLock<std::collections::HashMap<(u64, u64), serenity::Message>>> =
        Arc::new(RwLock::new(std::collections::HashMap::new()));
}

pub async fn on_message(msg: &serenity::Message) {
    if msg.author.bot || msg.guild_id.is_none() {
        return;
    }
    let mut msg_cache = DELETED_MSG_CACHE.write().await;
    msg_cache.insert(
        (msg.channel_id.get(), msg.id.get()),
        msg.clone(),
    );
    if msg_cache.len() > 1000 {
        // Simple eviction: remove oldest entries
        let keys_to_remove: Vec<_> = msg_cache.keys().take(500).cloned().collect();
        for key in keys_to_remove {
            msg_cache.remove(&key);
        }
    }
}

pub async fn on_message_delete(
    _ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    _guild_id: Option<serenity::GuildId>,
) {
    // Try to get the deleted message from cache
    // We store messages by channel_id + message_id for lookup
    let mut msg_cache = DELETED_MSG_CACHE.write().await;
    if let Some(msg) = msg_cache.remove(&(channel_id.get(), message_id.get())) {
        let mut cache = SNIPE_CACHE.write().await;
        let guild_id = msg.guild_id.map(|g| g.get()).unwrap_or(0);
        let guild_msgs = cache.entry(guild_id).or_default();
        guild_msgs.push(msg);
        if guild_msgs.len() > 50 {
            guild_msgs.remove(0);
        }
    }
}

/// makes the bot say a message
#[poise::command(slash_command, category = "fun")]
pub async fn say(
    ctx: Context<'_>,
    #[description = "message to say"] message: String,
) -> Result<(), Error> {
    let content = if message.contains("@everyone") || message.contains("@here") {
        format!("{} *nice job bud...*", message)
    } else {
        message
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("taizo says:")
                .description(&content)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// make the bot choose between options (separate with |)
#[poise::command(slash_command, category = "fun")]
pub async fn choose(
    ctx: Context<'_>,
    #[description = "choices separated by |"] choices: String,
) -> Result<(), Error> {
    let choicelist: Vec<&str> = choices.split(" | ").collect();

    if choicelist.len() < 2 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("you must define at least **2** choices!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if choicelist.len() > 10 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("the maximum amount of choices is **10**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let choice = choicelist.choose(&mut rand::thread_rng()).unwrap();
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("taizo chose:")
                .description(format!("```{}```", choice))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// hugs a member
#[poise::command(slash_command, category = "fun")]
pub async fn hug(
    ctx: Context<'_>,
    #[description = "member to hug"] member: serenity::Member,
) -> Result<(), Error> {
    use serenity::Mentionable;

    let images = [
        "https://i.pinimg.com/originals/51/fd/b2/51fdb2eaf2232753e5e4eac71d099091.gif",
        "https://acegif.com/wp-content/uploads/anime-hug.gif",
        "https://i.pinimg.com/originals/b6/2f/04/b62f047f8ed11b832cb6c0d8ec30687b.gif",
        "https://media1.tenor.com/images/5a273335be361bddb8fe464bf3b5bf05/tenor.gif?itemid=12668698",
        "https://media1.tenor.com/images/406a2179410010bd827d2764e3ea0cf1/tenor.gif?itemid=10200676",
    ];
    let hug_url = *images.choose(&mut rand::thread_rng()).unwrap();

    let description = if member.user.id == ctx.author().id {
        format!("*{} tried to hug themselves, +100 loneliness*", member.mention())
    } else if member.user.bot {
        format!("*Hugged {} (thx for the hug, uwu)*", member.mention())
    } else {
        format!("*Hugged {}*", member.mention())
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(&description)
                .image(hug_url)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// kisses a member
#[poise::command(slash_command, category = "fun")]
pub async fn kiss(
    ctx: Context<'_>,
    #[description = "member to kiss"] member: serenity::Member,
) -> Result<(), Error> {
    use serenity::Mentionable;

    let images = [
        "https://i.pinimg.com/originals/e3/4e/31/e34e31123f8f35d5c771a2d6a70bef52.gif",
        "https://media1.tenor.com/images/503bb007a3c84b569153dcfaaf9df46a/tenor.gif?itemid=17382412",
        "https://64.media.tumblr.com/5d51b3bbd64ccf1627dc87157a38e59f/tumblr_n5rfnvvj7H1t62gxao1_500.gif",
        "https://media2.giphy.com/media/bGm9FuBCGg4SY/giphy.gif",
        "https://media.tenor.com/images/fbb2b4d5c673ffcf8ec35e4652084c2a/tenor.gif",
        "https://i.pinimg.com/originals/32/d4/f0/32d4f0642ebb373e3eb072b2b91e6064.gif",
        "https://media1.tenor.com/images/ea9a07318bd8400fbfbd658e9f5ecd5d/tenor.gif?itemid=12612515",
    ];
    let kiss_url = *images.choose(&mut rand::thread_rng()).unwrap();

    let description = if member.user.id == ctx.author().id {
        "*How would you even do that?*".to_string()
    } else if member.user.bot {
        format!("*Kissed {} (thx for the kiss, uwu)*", member.mention())
    } else {
        format!("*Kissed {}*", member.mention())
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(&description)
                .image(kiss_url)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// sends a custom embed with your message
#[poise::command(slash_command, category = "fun")]
pub async fn embed(
    ctx: Context<'_>,
    #[description = "message for the embed"] msg: String,
) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(&msg)
                .author(
                    serenity::CreateEmbedAuthor::new(ctx.author().name.clone())
                        .icon_url(ctx.author().face()),
                )
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// rolls a six-sided die
#[poise::command(slash_command, category = "fun", aliases("roll"))]
pub async fn diceroll(ctx: Context<'_>) -> Result<(), Error> {
    let dice = rand::random::<u64>() % 6 + 1;
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!(":game_die: You rolled a **{}**", dice))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// see who eats the cookie first in a race against the clock
#[poise::command(slash_command, category = "fun")]
pub async fn cookie(ctx: Context<'_>) -> Result<(), Error> {
    use poise::serenity_prelude::Mentionable;

    let reply = ctx
        .send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("First one to eat the 🍪 wins!")
                    .color(0xF28080),
            ),
        )
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    reply
        .edit(
            ctx,
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(":three:")
                    .color(0xF28080),
            ),
        )
        .await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    reply
        .edit(
            ctx,
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(":two:")
                    .color(0xF28080),
            ),
        )
        .await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    reply
        .edit(
            ctx,
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(":one:")
                    .color(0xF2D380),
            ),
        )
        .await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let delay = rand::random::<u64>() % 3 + 1;
    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;

    reply
        .edit(
            ctx,
            poise::CreateReply::default()
                .embed(
                    serenity::CreateEmbed::new()
                        .description("Eat the cookie!")
                        .color(0xF28080),
                )
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new("cookie_eat")
                        .label("🍪 eat the cookie!")
                        .style(serenity::ButtonStyle::Danger),
                ])]),
        )
        .await?;

    let msg = reply.message().await?;

    let start = Instant::now();

    use poise::futures_util::StreamExt;

    let interaction = serenity::collector::ComponentInteractionCollector::new(&ctx.serenity_context().shard)
        .message_id(msg.id)
        .timeout(std::time::Duration::from_secs(60))
        .stream()
        .next()
        .await;

    match interaction {
        Some(interaction) => {
            let user_id = interaction.user.id;
            let elapsed = start.elapsed().as_secs_f64();
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default().embed(
                        serenity::CreateEmbed::new()
                            .description(format!(
                                "**{}** ate the cookie in `{:.3}` seconds!",
                                user_id.mention(),
                                elapsed
                            ))
                            .color(0x80F291),
                    ),
                )
                .await?;

            interaction
                .create_response(
                    &ctx,
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .components(vec![]),
                    ),
                )
                .await?;
        }
        None => {
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .embed(
                            serenity::CreateEmbed::new()
                                .description(":x: No one ate the cookie in time!")
                                .color(0xF28080),
                        )
                        .components(vec![]),
                )
                .await?;
        }
    }

    Ok(())
}

/// creates a poll with up to 20 choices (title | choice1 | choice2)
fn parse_duration(input: &str) -> Option<std::time::Duration> {
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        return Some(std::time::Duration::from_secs(86400));
    }
    let mut total = 0u64;
    let mut num = String::new();
    for c in input.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else {
            let n: u64 = num.parse().ok()?;
            total += match c {
                's' => n,
                'm' => n * 60,
                'h' => n * 3600,
                'd' => n * 86400,
                _ => return None,
            };
            num.clear();
        }
    }
    if total < 60 { None } else { Some(std::time::Duration::from_secs(total)) }
}

#[poise::command(slash_command, category = "fun")]
pub async fn poll(
    ctx: Context<'_>,
    #[description = "title | choice1 | choice2 ..."] choices: String,
    #[description = "expiry (e.g. 30m, 2h, 1d). default: 24h"] expiry: Option<String>,
) -> Result<(), Error> {
    let choicelist: Vec<&str> = choices.split(" | ").collect();

    if choicelist.len() < 3 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("you must define a title and at least two choices!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if choicelist.len() > 21 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("the maximum amount of choices is **20**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let duration = match expiry.as_deref().and_then(parse_duration) {
        Some(d) => d,
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("invalid duration! use formats like `30m`, `2h`, `1d`")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let choice_labels = [
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10",
        "11", "12", "13", "14", "15", "16", "17", "18", "19", "20",
    ];
    let desc: String = choicelist[1..]
        .iter()
        .enumerate()
        .map(|(i, c)| format!("**{}**) {}", choice_labels[i], c))
        .collect::<Vec<_>>()
        .join("\n");

    let buttons: Vec<serenity::CreateButton> = choicelist[1..]
        .iter()
        .enumerate()
        .map(|(i, c)| {
            serenity::CreateButton::new(format!("poll_{}", i))
                .label(format!("{} {}", choice_labels[i], c))
                .style(serenity::ButtonStyle::Primary)
        })
        .collect();

    let expires_at = chrono::Utc::now() + chrono::Duration::from_std(duration)
        .map_err(|e| -> Error { e.into() })?;

    let reply = ctx.send(
        poise::CreateReply::default()
            .embed(
                serenity::CreateEmbed::new()
                    .title(choicelist[0])
                    .description(&desc)
                    .footer(
                        serenity::CreateEmbedFooter::new(format!(
                            "poll made by {} • expires {}",
                            ctx.author().name,
                            expires_at.format("%b %d, %Y %H:%M UTC")
                        ))
                        .icon_url(ctx.author().face()),
                    )
                    .timestamp(serenity::Timestamp::now())
                    .color(0xF28080),
            )
            .components(vec![serenity::CreateActionRow::Buttons(buttons)]),
    )
    .await?;

    let msg = reply.message().await?;
    sqlx::query("INSERT INTO polls (message_id, channel_id, user_id, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(msg.id.get() as i64)
        .bind(ctx.channel_id().get() as i64)
        .bind(ctx.author().id.get() as i64)
        .bind(expires_at)
        .execute(&ctx.data().db)
        .await?;

    let http = ctx.serenity_context().http.clone();
    let db = ctx.data().db.clone();
    let channel_id = ctx.channel_id();
    let message_id = msg.id;
    let poll_title = choicelist[0].to_string();
    let poll_choices: Vec<String> = choicelist[1..].iter().map(|s| s.to_string()).collect();

    tokio::spawn(async move {
        tokio::time::sleep(duration).await;

        let votes: Vec<(i32, i64)> = sqlx::query_as(
            "SELECT choice_index, COUNT(*) as count FROM poll_votes WHERE message_id = $1 GROUP BY choice_index ORDER BY choice_index",
        )
        .bind(message_id.get() as i64)
        .fetch_all(&db)
        .await
        .unwrap_or_default();

        let vote_map: std::collections::HashMap<i32, i64> = votes.into_iter().collect();
        let total_votes: i64 = vote_map.values().sum();

        let results: String = poll_choices
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let count = vote_map.get(&(i as i32)).copied().unwrap_or(0);
                let bar_len = if total_votes > 0 {
                    (count as f64 / total_votes as f64 * 10.0).round() as usize
                } else {
                    0
                };
                let bar = "█".repeat(bar_len);
                format!("**{}**) {} — {} vote{} {}", i + 1, c, count, if count == 1 { "" } else { "s" }, bar)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut components = Vec::new();
        for chunk in poll_choices.chunks(5) {
            let buttons: Vec<serenity::CreateButton> = chunk
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let global_idx = components.len() * 5 + i;
                    serenity::CreateButton::new(format!("poll_{}", global_idx))
                        .label(format!("{} {}", global_idx + 1, c))
                        .style(serenity::ButtonStyle::Primary)
                        .disabled(true)
                })
                .collect();
            components.push(serenity::CreateActionRow::Buttons(buttons));
        }

        let _ = http.edit_message(
            channel_id,
            message_id,
            &serenity::EditMessage::new()
                .embed(
                    serenity::CreateEmbed::new()
                        .title(&poll_title)
                        .description(results)
                        .footer(serenity::CreateEmbedFooter::new(format!("poll ended • {} total vote{}", total_votes, if total_votes == 1 { "" } else { "s" })))
                        .color(0xF28080),
                )
                .components(components),
            Vec::<serenity::CreateAttachment>::new(),
        ).await;
    });

    Ok(())
}

/// create a yes or no poll
#[poise::command(slash_command, category = "fun")]
pub async fn yesno(
    ctx: Context<'_>,
    #[description = "title for the poll"] title: String,
) -> Result<(), Error> {
    let reply = ctx
        .send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .title(&title)
                    .description("👍 Yes\n👎 No")
                    .footer(
                        serenity::CreateEmbedFooter::new(format!(
                            "poll made by {}",
                            ctx.author().name
                        ))
                        .icon_url(ctx.author().face()),
                    )
                    .timestamp(serenity::Timestamp::now())
                    .color(0xF28080),
            ),
        )
        .await?;

    let msg = reply.message().await?;

    msg.react(&ctx, serenity::ReactionType::Unicode("👍".to_string()))
        .await?;
    msg.react(&ctx, serenity::ReactionType::Unicode("👎".to_string()))
        .await?;

    Ok(())
}

pub async fn handle_poll_button(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    db: &sqlx::PgPool,
) -> Result<(), Error> {
    let clicked_id = &interaction.data.custom_id;
    let choice_index: i32 = clicked_id.strip_prefix("poll_")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let msg_id = interaction.message.id.get() as i64;
    let user_id = interaction.user.id.get() as i64;

    let expired: Option<bool> = sqlx::query_scalar("SELECT expires_at < NOW() FROM polls WHERE message_id = $1")
        .bind(msg_id)
        .fetch_optional(db)
        .await?;

    if expired == Some(true) {
        let poll_title = interaction
            .message
            .embeds
            .first()
            .and_then(|e| e.title.as_deref())
            .unwrap_or("poll")
            .to_string();

        let choices: Vec<String> = interaction
            .message
            .embeds
            .first()
            .and_then(|e| e.description.as_deref())
            .map(|desc| {
                desc.lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        let after = line.split("**) ").nth(1)?;
                        Some(after.trim().to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();

        let votes: Vec<(i32, i64)> = sqlx::query_as(
            "SELECT choice_index, COUNT(*) as count FROM poll_votes WHERE message_id = $1 GROUP BY choice_index ORDER BY choice_index",
        )
        .bind(msg_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

        let vote_map: std::collections::HashMap<i32, i64> = votes.into_iter().collect();
        let total_votes: i64 = vote_map.values().sum();

        let results: String = choices
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let count = vote_map.get(&(i as i32)).copied().unwrap_or(0);
                let bar_len = if total_votes > 0 {
                    (count as f64 / total_votes as f64 * 10.0).round() as usize
                } else {
                    0
                };
                let bar = "█".repeat(bar_len);
                format!("**{}**) {} — {} vote{} {}", i + 1, c, count, if count == 1 { "" } else { "s" }, bar)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let disabled_components: Vec<serenity::CreateActionRow> = choices
            .iter()
            .enumerate()
            .map(|(i, choice)| {
                serenity::CreateButton::new(format!("poll_{}", i))
                    .label(format!("{} {}", i + 1, choice))
                    .style(serenity::ButtonStyle::Primary)
                    .disabled(true)
            })
            .collect::<Vec<_>>()
            .chunks(5)
            .map(|chunk| serenity::CreateActionRow::Buttons(chunk.to_vec()))
            .collect();

        interaction
            .create_response(
                ctx,
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(
                            serenity::CreateEmbed::new()
                                .title(&poll_title)
                                .description(results)
                                .footer(serenity::CreateEmbedFooter::new(format!("poll ended • {} total vote{}", total_votes, if total_votes == 1 { "" } else { "s" })))
                                .color(0xF28080),
                        )
                        .components(disabled_components),
                ),
            )
            .await?;
        return Ok(());
    }

    let choice_text = interaction
        .message
        .embeds
        .first()
        .and_then(|e| e.description.as_deref())
        .and_then(|desc| {
            desc.lines().nth(choice_index as usize).and_then(|line| {
                line.trim().split("**) ").nth(1).map(|s| s.trim().to_string())
            })
        })
        .unwrap_or_else(|| format!("option {}", choice_index + 1));

    let existing: Option<i32> = sqlx::query_scalar("SELECT choice_index FROM poll_votes WHERE message_id = $1 AND user_id = $2")
        .bind(msg_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?;

    if let Some(prev) = existing {
        if prev == choice_index {
            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content(format!("you already voted for **{}**!", choice_text))
                            .ephemeral(true),
                    ),
                )
                .await?;
            return Ok(());
        }
    }

    sqlx::query("INSERT INTO poll_votes (message_id, user_id, choice_index) VALUES ($1, $2, $3) ON CONFLICT (message_id, user_id) DO UPDATE SET choice_index = $3")
        .bind(msg_id)
        .bind(user_id)
        .bind(choice_index)
        .execute(db)
        .await?;

    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(format!("✅ you voted for **{}**!", choice_text))
                    .ephemeral(true),
            ),
        )
        .await?;

    Ok(())
}

async fn fetch_meme(subreddit: &str) -> Result<serde_json::Value, reqwest::Error> {
    let client = reqwest::Client::new();
    client
        .get(format!("https://meme-api.com/gimme/{}", subreddit))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await
}

/// fetches a random meme from r/memes
#[poise::command(slash_command, category = "fun")]
pub async fn meme(ctx: Context<'_>) -> Result<(), Error> {
    let res = match fetch_meme("memes").await {
        Ok(r) => r,
        Err(_) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("could not fetch memes right now")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let title = res["title"].as_str().unwrap_or("meme");
    let url = res["url"].as_str().unwrap_or("");
    let post_link = res["postLink"].as_str().unwrap_or("");
    let score = res["ups"].as_i64().unwrap_or(0);

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(title)
                .url(post_link)
                .image(url)
                .footer(serenity::CreateEmbedFooter::new(format!("⬆️ {}", score)))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// fetches a random dank meme from r/dankmemes
#[poise::command(slash_command, category = "fun")]
pub async fn dankmeme(ctx: Context<'_>) -> Result<(), Error> {
    let res = match fetch_meme("dankmemes").await {
        Ok(r) => r,
        Err(_) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("could not fetch memes right now")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let title = res["title"].as_str().unwrap_or("dank meme");
    let url = res["url"].as_str().unwrap_or("");
    let post_link = res["postLink"].as_str().unwrap_or("");
    let score = res["ups"].as_i64().unwrap_or(0);

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(title)
                .url(post_link)
                .image(url)
                .footer(serenity::CreateEmbedFooter::new(format!("⬆️ {}", score)))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// fetches a random post from r/ProgrammerHumor
#[poise::command(slash_command, category = "fun", aliases("ph"))]
pub async fn programmerhumor(ctx: Context<'_>) -> Result<(), Error> {
    let res = match fetch_meme("ProgrammerHumor").await {
        Ok(r) => r,
        Err(_) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("could not fetch posts right now")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    let title = res["title"].as_str().unwrap_or("post");
    let url = res["url"].as_str().unwrap_or("");
    let post_link = res["postLink"].as_str().unwrap_or("");
    let score = res["ups"].as_i64().unwrap_or(0);

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(title)
                .url(post_link)
                .image(url)
                .footer(serenity::CreateEmbedFooter::new(format!("⬆️ {}", score)))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// tells you a random dad joke
#[poise::command(slash_command, category = "fun", aliases("djoke"))]
pub async fn dadjoke(ctx: Context<'_>) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let res = client
        .get("https://icanhazdadjoke.com/")
        .header("Accept", "application/json")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let joke = res["joke"].as_str().unwrap_or("no joke found");

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("here's a random dad joke")
                .description(joke)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// fetches a random post from a subreddit
#[poise::command(slash_command, category = "fun")]
pub async fn reddit(
    ctx: Context<'_>,
    #[description = "subreddit name"] subreddit: String,
) -> Result<(), Error> {
    let skip = ["memes", "dankmemes", "programmerhumor"];
    if skip.contains(&subreddit.as_str()) {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!(
                        "this subreddit has its own command! do **/{}**",
                        subreddit.trim_end_matches('s')
                    ))
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let res = match fetch_meme(&subreddit).await {
        Ok(r) => r,
        Err(_) => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("could not find that subreddit!")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    };

    if res["nsfw"].as_bool().unwrap_or(false) {
        let is_nsfw = ctx.channel_id().to_channel(&ctx).await.map(|c| {
            if let serenity::Channel::Guild(gc) = c {
                gc.nsfw
            } else {
                false
            }
        }).unwrap_or(false);

        if !is_nsfw {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("that subreddit/post is marked as **NSFW!**")
                        .color(0xF28080),
                ),
            )
            .await?;
            return Ok(());
        }
    }

    let title = res["title"].as_str().unwrap_or("post");
    let url = res["url"].as_str().unwrap_or("");
    let post_link = res["postLink"].as_str().unwrap_or("");
    let score = res["ups"].as_i64().unwrap_or(0);

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(format!("r/{}", subreddit))
                .url(post_link)
                .description(title)
                .image(url)
                .footer(serenity::CreateEmbedFooter::new(format!("⬆️ {}", score)))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// ask the magical 8ball a question
#[poise::command(slash_command, category = "fun", aliases("8ball"))]
pub async fn eightball(
    ctx: Context<'_>,
    #[description = "question to ask"] message: String,
) -> Result<(), Error> {
    if !message.ends_with('?') {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("your response must be in the form of a question!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let responses = [
        "As I see it, yes.",
        "Ask again later.",
        "Better not tell you now.",
        "Cannot predict now.",
        "Concentrate and ask again.",
        "Don't count on it.",
        "It is certain.",
        "It is decidedly so.",
        "Most likely.",
        "My reply is no.",
        "My sources say no.",
        "Outlook not so good.",
        "Outlook good.",
        "Reply hazy, try again.",
        "Signs point to yes.",
        "Very doubtful.",
        "Without a doubt.",
        "Yes.",
        "Yes – definitely.",
        "You may rely on it.",
    ];

    let choice = responses.choose(&mut rand::thread_rng()).unwrap();

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(":8ball: 8 Ball says:")
                .description(*choice)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// sends the original link to a custom emoji
#[poise::command(slash_command, category = "fun")]
pub async fn enlarge(
    ctx: Context<'_>,
    #[description = "emoji to enlarge"] emoji: serenity::Emoji,
) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(&emoji.name)
                .url(emoji.url())
                .image(emoji.url())
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// measures your dong size
#[poise::command(slash_command, category = "fun", aliases("pp"))]
pub async fn dong(
    ctx: Context<'_>,
    #[description = "member to measure"] member: Option<serenity::Member>,
) -> Result<(), Error> {
    let member = match member {
        Some(m) => m,
        None => {
            let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
            guild_id.member(&ctx, ctx.author().id).await?
        }
    };
    let dsize = rand::random::<u64>() % 20 + 1;
    let pp = format!("8{}D", "=".repeat(dsize as usize));

    let response = if dsize <= 5 {
        format!("{} you might need penis enlargement pills", member.user.name)
    } else if dsize <= 10 {
        format!("pretty average {}, you got game", member.user.name)
    } else if dsize <= 15 {
        format!(
            "that's a big dick you got there {}, you got hella game",
            member.user.name
        )
    } else {
        format!(
            "WOO {}, YOU'RE PACKING!!",
            member.user.name.to_uppercase()
        )
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(&response)
                .description(format!("{}'s dong size:\n**{}**", member.user.name, pp))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// make the bot toast you with a compliment
#[poise::command(slash_command, category = "fun")]
pub async fn toast(
    ctx: Context<'_>,
    #[description = "member to toast"] member: Option<serenity::Member>,
) -> Result<(), Error> {
    let member = match member {
        Some(m) => m,
        None => {
            let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
            guild_id.member(&ctx, ctx.author().id).await?
        }
    };

    let toasts = [
        "Your hair is looking awesome today!",
        "Your clothes really compliment the color of your eyes!",
        "Your positivity is infectious.",
        "You should be so proud of yourself.",
        "You're amazing!",
        "You have a remarkable sense of humor.",
        "You are one of a kind.",
        "You are beautiful inside and out.",
        "You are so strong.",
        "Your mere presence is reassuring to me.",
        "You deserve everything you've achieved.",
        "You have a good head on your shoulders.",
        "You are wise beyond your years.",
        "Never stop being you!",
        "You make the small things count.",
        "You are a ray of sunshine.",
        "You always know how to find the silver lining.",
        "Is there anything you can't do!?",
        "You're so unique.",
        "You are making a real difference in the world.",
        "Your potential is limitless.",
        "Your heart must be 10 times the average size.",
        "Thanks for being you!",
        "You are such a good listener.",
        "Your capacity for generosity knows no bounds.",
        "You have such a great heart.",
        "You're a constant reminder that people can be good.",
        "You have the best ideas.",
        "You're the most perfect 'you' there is.",
        "You are the epitome of a good person.",
        "You're the person that everyone wants on their team.",
        "You continue to impress me.",
        "You are so special to everyone you know.",
        "Thank you for being such a great person.",
        "The way you carry yourself is truly admirable.",
        "You set a great example for everyone around you.",
        "You are so down to earth.",
        "Your parents must be so proud.",
        "How did you learn to be so great?",
        "You have the courage of your convictions.",
        "On a scale of one to ten, you're an eleven.",
    ];

    let response = toasts.choose(&mut rand::thread_rng()).unwrap();

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("toast")
                .description(format!("**{}**, {}", member.user.name, response))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// OWO-ify some text
#[poise::command(slash_command, category = "fun", aliases("owo"))]
pub async fn owoify(
    ctx: Context<'_>,
    #[description = "text to owoify"] text: String,
) -> Result<(), Error> {
    let owo = text
        .replace('l', "w")
        .replace('L', "W")
        .replace('r', "w")
        .replace('R', "W")
        .replace("th", "d")
        .replace("ove", "uv");

    let suffixes = ["XwX", "OvO", "OwO", "UwU", ">:3", "-w-", "ÙwÚ"];
    let suffix = suffixes.choose(&mut rand::thread_rng()).unwrap();

    ctx.say(format!("{} {}", owo, suffix)).await?;
    Ok(())
}

/// ask taizo a yes or no question
#[poise::command(slash_command, category = "fun")]
pub async fn yn(
    ctx: Context<'_>,
    #[description = "question to ask"] question: String,
) -> Result<(), Error> {
    let choices = ["Yes", "No"];
    let choice = choices.choose(&mut rand::thread_rng()).unwrap();

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("yes or no?")
                .field("you asked:", &question, false)
                .field("taizo says:", *choice, false)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// shows a recently deleted message
#[poise::command(slash_command, category = "fun")]
pub async fn snipe(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?.get();
    let cache = SNIPE_CACHE.read().await;

    match cache.get(&guild_id) {
        Some(msgs) if !msgs.is_empty() => {
            let msg = &msgs[msgs.len() - 1];
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description(&msg.content)
                        .author(
                            serenity::CreateEmbedAuthor::new(msg.author.name.clone())
                                .icon_url(msg.author.face()),
                        )
                        .timestamp(msg.timestamp)
                        .color(0xF28080),
                ),
            )
            .await?;
        }
        _ => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("there's no messages to show!")
                        .color(0xF28080),
                ),
            )
            .await?;
        }
    }

    Ok(())
}
