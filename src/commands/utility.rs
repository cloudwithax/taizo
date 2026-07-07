use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::collections::HashMap;

struct CogCommands {
    name: String,
    commands: Vec<(&'static str, &'static str)>,
}

fn all_cogs() -> Vec<CogCommands> {
    vec![
        CogCommands {
            name: "utility".into(),
            commands: vec![
                ("/ping", "bot latency"),
                ("/help", "show all commands"),
                ("/serverinfo", "server info"),
                ("/userinfo", "user info"),
                ("/avatar", "member avatar"),
                ("/whois", "detailed member info"),
                ("/servericon", "server icon"),
            ],
        },
        CogCommands {
            name: "moderation".into(),
            commands: vec![
                ("/ban", "ban a user"),
                ("/kick", "kick a user"),
                ("/mute", "timeout a user"),
                ("/unmute", "remove timeout"),
                ("/warn", "warn a user"),
                ("/warnings", "view warnings"),
                ("/unban", "unban a user"),
                ("/purge", "delete messages"),
                ("/setwelcome", "set welcome message"),
                ("/setleave", "set leave message"),
            ],
        },
        CogCommands {
            name: "fun".into(),
            commands: vec![
                ("/say", "bot says something"),
                ("/choose", "pick from choices"),
                ("/hug", "hug a member"),
                ("/kiss", "kiss a member"),
                ("/embed", "custom embed"),
                ("/diceroll", "roll a die"),
                ("/cookie", "cookie race"),
                ("/poll", "create a poll"),
                ("/yesno", "yes or no poll"),
                ("/meme", "random meme"),
                ("/dankmeme", "random dank meme"),
                ("/programmerhumor", "programmer humor"),
                ("/dadjoke", "random dad joke"),
                ("/reddit", "random subreddit post"),
                ("/eightball", "ask 8ball"),
                ("/enlarge", "enlarge emoji"),
                ("/dong", "measure dong"),
                ("/toast", "give a toast"),
                ("/owoify", "owo-ify text"),
                ("/yn", "yes or no"),
                ("/snipe", "deleted messages"),
            ],
        },
        CogCommands {
            name: "info".into(),
            commands: vec![
                ("/about", "bot info"),
                ("/uptime", "bot uptime"),
                ("/invite", "bot invite"),
                ("/privacy", "privacy policy"),
                ("/vote", "vote for bot"),
                ("/support", "support server"),
            ],
        },
        CogCommands {
            name: "economy".into(),
            commands: vec![
                ("/openaccount", "create a bank account"),
                ("/closeaccount", "delete your account"),
                ("/balance", "check your balance"),
                ("/work", "earn money"),
                ("/slut", "risky work"),
                ("/crime", "commit a crime"),
                ("/daily", "daily bonus"),
                ("/weekly", "weekly bonus"),
                ("/deposit", "wallet → bank"),
                ("/depositall", "deposit all"),
                ("/withdraw", "bank → wallet"),
                ("/withdrawall", "withdraw all"),
                ("/pay", "send money"),
                ("/coinflip", "bet on a coin flip"),
                ("/highlow", "guess higher or lower"),
                ("/leaderboard", "top 10 richest"),
            ],
        },
    ]
}

const PER_PAGE: usize = 10;

fn build_cog_buttons() -> Vec<serenity::CreateButton> {
    let cogs = all_cogs();
    cogs.iter()
        .map(|c| {
            serenity::CreateButton::new(format!("help_cog_{}", c.name))
                .label(&c.name)
                .style(serenity::ButtonStyle::Secondary)
        })
        .collect()
}

fn build_page_buttons(cog_idx: usize, page: usize, total_pages: usize) -> Vec<serenity::CreateActionRow> {
    let mut rows = Vec::new();

    // Category buttons row
    rows.push(serenity::CreateActionRow::Buttons(build_cog_buttons()));

    // Pagination row (only if more than 1 page)
    if total_pages > 1 {
        let mut pagination_buttons = Vec::new();
        pagination_buttons.push(
            serenity::CreateButton::new(format!("help_page_{}_prev", cog_idx))
                .label("← prev")
                .style(serenity::ButtonStyle::Primary)
                .disabled(page == 0),
        );
        pagination_buttons.push(
            serenity::CreateButton::new(format!("help_page_info"))
                .label(format!("{} / {}", page + 1, total_pages))
                .style(serenity::ButtonStyle::Secondary)
                .disabled(true),
        );
        pagination_buttons.push(
            serenity::CreateButton::new(format!("help_page_{}_next", cog_idx))
                .label("next →")
                .style(serenity::ButtonStyle::Primary)
                .disabled(page + 1 >= total_pages),
        );
        rows.push(serenity::CreateActionRow::Buttons(pagination_buttons));
    }

    rows
}

fn build_embed(cog: &CogCommands, page: usize) -> serenity::CreateEmbed {
    let start = page * PER_PAGE;
    let end = std::cmp::min(start + PER_PAGE, cog.commands.len());
    let slice = &cog.commands[start..end];

    let total_pages = (cog.commands.len() + PER_PAGE - 1) / PER_PAGE;
    let total_commands: usize = all_cogs().iter().map(|c| c.commands.len()).sum();

    let description: Vec<String> = slice
        .iter()
        .map(|(cmd, desc)| format!("**{}**\n{}", cmd, desc))
        .collect();

    serenity::CreateEmbed::new()
        .title(format!("page {}/{} ({} commands)", page + 1, total_pages, total_commands))
        .description(format!("**{} Commands**\n\n{}\n\n*use \";help command\" for more info on a command.*", cog.name, description.join("\n")))
        .color(0xF28080)
}

/// show all commands
#[poise::command(slash_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let cogs = all_cogs();
    let first_cog = &cogs[0];
    let total_pages = (first_cog.commands.len() + PER_PAGE - 1) / PER_PAGE;

    ctx.send(
        poise::CreateReply::default()
            .embed(build_embed(first_cog, 0))
            .components(build_page_buttons(0, 0, total_pages)),
    )
    .await?;
    Ok(())
}

pub async fn handle_help_button(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
) -> Result<(), Error> {
    let custom_id = &interaction.data.custom_id;

    let cogs = all_cogs();
    let mut cog_map: HashMap<String, usize> = HashMap::new();
    for (i, c) in cogs.iter().enumerate() {
        cog_map.insert(c.name.clone(), i);
    }

    if custom_id == "help_page_info" {
        interaction.defer(ctx).await.ok();
        return Ok(());
    }

    // Category button
    if let Some(name) = custom_id.strip_prefix("help_cog_") {
        if let Some(&cog_idx) = cog_map.get(name) {
            let cog = &cogs[cog_idx];
            let total_pages = (cog.commands.len() + PER_PAGE - 1) / PER_PAGE;
            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(build_embed(cog, 0))
                            .components(build_page_buttons(cog_idx, 0, total_pages)),
                    ),
                )
                .await?;
            return Ok(());
        }
    }

    // Page buttons
    if let Some(rest) = custom_id.strip_prefix("help_page_") {
        if let Some(cog_idx_str) = rest.strip_suffix("_prev") {
            let cog_idx: usize = cog_idx_str.parse().unwrap_or(0);
            // Find current page from the embed title
            let current_page = extract_current_page(interaction);
            let new_page = current_page.saturating_sub(1);
            let cog = &cogs[cog_idx];
            let total_pages = (cog.commands.len() + PER_PAGE - 1) / PER_PAGE;
            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(build_embed(cog, new_page))
                            .components(build_page_buttons(cog_idx, new_page, total_pages)),
                    ),
                )
                .await?;
            return Ok(());
        }
        if let Some(cog_idx_str) = rest.strip_suffix("_next") {
            let cog_idx: usize = cog_idx_str.parse().unwrap_or(0);
            let current_page = extract_current_page(interaction);
            let new_page = current_page + 1;
            let cog = &cogs[cog_idx];
            let total_pages = (cog.commands.len() + PER_PAGE - 1) / PER_PAGE;
            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(build_embed(cog, new_page))
                            .components(build_page_buttons(cog_idx, new_page, total_pages)),
                    ),
                )
                .await?;
            return Ok(());
        }
    }

    Ok(())
}

fn extract_current_page(interaction: &serenity::ComponentInteraction) -> usize {
    let embeds = &interaction.message.embeds;
    if let Some(embed) = embeds.first() {
        if let Some(title) = &embed.title {
            if let Some(page_str) = title.split("— page ").nth(1) {
                if let Some(page_num) = page_str.split('/').next() {
                    if let Ok(p) = page_num.parse::<usize>() {
                        return p - 1;
                    }
                }
            }
        }
    }
    0
}

/// check the bot's latency
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let latency = ctx.ping().await;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(format!("{}ms", latency.as_millis()))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// get information about the server
#[poise::command(slash_command)]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    use poise::serenity_prelude::Mentionable;

    let (guild_name, icon_url, owner_id, member_count, created_at) = {
        let guild = ctx.guild().ok_or("command must be used in a guild")?;
        (
            guild.name.clone(),
            guild.icon_url(),
            guild.owner_id,
            guild.member_count,
            guild.id.created_at().timestamp(),
        )
    };

    let owner = owner_id.to_user(&ctx).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(&guild_name)
                .thumbnail(
                    icon_url
                        .unwrap_or_else(|| "https://cdn.discordapp.com/embed/avatars/0.png".to_string()),
                )
                .description(format!(
                    "{} members\nowned by {}\ncreated <t:{}:F>",
                    member_count,
                    owner.mention(),
                    created_at,
                ))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// get information about a user
#[poise::command(slash_command)]
pub async fn userinfo(
    ctx: Context<'_>,
    #[description = "user to get info about"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let user = user.unwrap_or_else(|| ctx.author().clone());

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(&user.name)
                .thumbnail(user.face())
                .description(format!(
                    "{}\ncreated <t:{}:F>",
                    if user.bot { "bot" } else { "user" },
                    user.created_at().timestamp(),
                ))
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// shows a member's avatar
#[poise::command(slash_command, aliases("av"))]
pub async fn avatar(
    ctx: Context<'_>,
    #[description = "member to get avatar of"] member: Option<serenity::Member>,
) -> Result<(), Error> {
    let member = match member {
        Some(m) => m,
        None => {
            let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
            guild_id.member(&ctx, ctx.author().id).await?
        }
    };
    let avatar = member.face();

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("avatar")
                .url(&avatar)
                .image(&avatar)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// shows useful info about a member
#[poise::command(slash_command, aliases("ui"))]
pub async fn whois(
    ctx: Context<'_>,
    #[description = "member to get info about"] member: Option<serenity::Member>,
) -> Result<(), Error> {
    use poise::serenity_prelude::Mentionable;

    let member = match member {
        Some(m) => m,
        None => {
            let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
            guild_id.member(&ctx, ctx.author().id).await?
        }
    };

    let guild_roles = member.roles(&ctx).unwrap_or_default();
    let roles: Vec<String> = guild_roles
        .iter()
        .map(|r| format!("<@&{}>", r.id))
        .collect();

    let role_count = roles.len();
    let roles_str = if role_count == 0 {
        "None".to_string()
    } else {
        roles.join(" ")
    };

    let guild_id = ctx.guild_id().ok_or("must be used in a guild")?;
    let perms: Vec<String> = if let Some(guild) = ctx.cache().guild(guild_id) {
        if let Some(channel) = guild.channels.get(&ctx.channel_id()) {
            guild.user_permissions_in(channel, &member)
                .iter()
                .map(|p| format!("{}", p).replace('_', " "))
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let perm_count = perms.len();
    let perms_str = perms.join(", ");

    let joined = member
        .joined_at
        .map(|t| format!("<t:{}:F>", t.timestamp()))
        .unwrap_or_else(|| "unknown".to_string());

    let created = format!("<t:{}:F>", member.user.created_at().timestamp());

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(member.mention().to_string())
                .author(
                    serenity::CreateEmbedAuthor::new(member.user.name.clone())
                        .icon_url(member.face()),
                )
                .thumbnail(member.face())
                .field("joined", &joined, true)
                .field("registered", &created, true)
                .field("user id", member.user.id.to_string(), false)
                .field(format!("roles [{}]", role_count), &roles_str, false)
                .field(
                    format!("permissions [{}]", perm_count),
                    &perms_str,
                    false,
                )
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// sends the server's icon
#[poise::command(slash_command, aliases("svi"))]
pub async fn servericon(ctx: Context<'_>) -> Result<(), Error> {
    let icon = {
        let guild = ctx.guild().ok_or("must be used in a guild")?;
        guild
            .icon_url()
            .unwrap_or_else(|| "https://cdn.discordapp.com/embed/avatars/0.png".to_string())
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("server icon")
                .url(&icon)
                .image(&icon)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}
