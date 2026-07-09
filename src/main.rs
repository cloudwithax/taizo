mod commands;

use poise::serenity_prelude as serenity;
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info};
use serenity::Mentionable;

pub struct Data {
    _start_time: std::time::Instant,
    pub db: sqlx::PgPool,
}

struct DbKey;

impl serenity::prelude::TypeMapKey for DbKey {
    type Value = sqlx::PgPool;
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let token = std::env::var("TOKEN").expect("missing TOKEN");
    let database_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let intents = serenity::GatewayIntents::non_privileged();

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    for statement in include_str!("../schema.sql").split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed)
                .execute(&db)
                .await
                .expect("failed to run schema");
        }
    }

    info!("database connected and schema initialized");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::utility::help(),
                commands::utility::ping(),
                commands::utility::serverinfo(),
                commands::utility::userinfo(),
                commands::utility::avatar(),
                commands::utility::whois(),
                commands::utility::servericon(),
                commands::moderation::ban(),
                commands::moderation::kick(),
                commands::moderation::mute(),
                commands::moderation::unmute(),
                commands::moderation::warn(),
                commands::moderation::unban(),
                commands::moderation::purge(),
                commands::moderation::warnings(),
                commands::moderation::setwelcome(),
                commands::moderation::setleave(),
                commands::moderation::honeypot(),
                commands::fun::say(),
                commands::fun::choose(),
                commands::fun::hug(),
                commands::fun::kiss(),
                commands::fun::embed(),
                commands::fun::diceroll(),
                commands::fun::cookie(),
                commands::fun::poll(),
                commands::fun::yesno(),
                commands::fun::meme(),
                commands::fun::dankmeme(),
                commands::fun::programmerhumor(),
                commands::fun::dadjoke(),
                commands::fun::reddit(),
                commands::fun::eightball(),
                commands::fun::enlarge(),
                commands::fun::dong(),
                commands::fun::toast(),
                commands::fun::owoify(),
                commands::fun::yn(),
                commands::fun::snipe(),
                commands::info::about(),
                commands::info::uptime(),
                commands::info::invite(),
                commands::info::privacy(),
                commands::info::vote(),
                commands::info::support(),
                commands::economy::openaccount(),
                commands::economy::closeaccount(),
                commands::economy::balance(),
                commands::economy::work(),
                commands::economy::slut(),
                commands::economy::crime(),
                commands::economy::daily(),
                commands::economy::weekly(),
                commands::economy::deposit(),
                commands::economy::depositall(),
                commands::economy::withdraw(),
                commands::economy::withdrawall(),
                commands::economy::pay(),
                commands::economy::coinflip(),
                commands::economy::highlow(),
                commands::economy::blackjack(),
                commands::economy::leaderboard(),
                commands::reactionrole::reactionrole(),
                commands::ticket::ticket(),
                commands::auditlog::auditlog(),
                commands::owner::restart(),
                commands::owner::stop(),
                commands::emoji::steal(),
            ],
            on_error: |error| {
                Box::pin(async move {
                    match &error {
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            error!("Command error: {:?}", error);
                            let _ = ctx
                                .say(format!("command failed: {:?}", error))
                                .await;
                        }
                        poise::FrameworkError::MissingBotPermissions { missing_permissions, ctx, .. } => {
                            error!("Missing bot permissions: {:?}", missing_permissions);
                            let _ = ctx
                                .say(format!("bot missing permissions: {}", missing_permissions))
                                .await;
                        }
                        poise::FrameworkError::MissingUserPermissions { missing_permissions, ctx, .. } => {
                            error!("Missing user permissions: {:?}", missing_permissions);
                            let _ = ctx
                                .say("you don't have permission to use this command")
                                .await;
                        }
                        other => {
                            error!("Unhandled framework error: {}", other);
                        }
                    }
                })
            },
            event_handler: |ctx, event, framework, _data| {
                Box::pin(async move {
                    match event {
                        serenity::FullEvent::Message { new_message } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                commands::moderation::handle_honeypot_message(&ctx.http, &db, new_message).await;
                            }
                            commands::fun::on_message(new_message).await;
                        }
                        serenity::FullEvent::MessageDelete {
                            channel_id,
                            deleted_message_id,
                            guild_id,
                        } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                if let Some(gid) = guild_id {
                                    let embed = serenity::CreateEmbed::new()
                                        .title("message deleted")
                                        .field("channel", format!("<#{}>", channel_id), false)
                                        .field("author", format!("<@{}>", deleted_message_id), false)
                                        .color(0xF28080)
                                        .timestamp(chrono::Utc::now());
                                    commands::auditlog::log_event(&ctx.http, &db, gid.get() as i64, "messages", embed).await;
                                }
                            }
                            commands::fun::on_message_delete(
                                ctx,
                                *channel_id,
                                *deleted_message_id,
                                *guild_id,
                            )
                            .await;
                        }
                        serenity::FullEvent::ReactionAdd { add_reaction } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                commands::reactionrole::handle_reaction_add(ctx, add_reaction, &db).await;
                            }
                        }
                        serenity::FullEvent::ReactionRemove { removed_reaction } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                commands::reactionrole::handle_reaction_remove(ctx, removed_reaction, &db).await;
                            }
                        }
                        serenity::FullEvent::MessageUpdate { old_if_available, new, .. } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                if let Some(msg) = new {
                                    if let Some(gid) = msg.guild_id {
                                        let before = old_if_available.as_ref().map(|m| m.content.as_str()).unwrap_or("unknown");
                                        let embed = serenity::CreateEmbed::new()
                                            .title("message edited")
                                            .field("channel", format!("<#{}>", msg.channel_id), false)
                                            .field("author", msg.author.mention().to_string(), false)
                                            .field("before", before, false)
                                            .field("after", &msg.content, false)
                                            .color(0xF2D380)
                                            .timestamp(chrono::Utc::now());
                                        commands::auditlog::log_event(&ctx.http, &db, gid.get() as i64, "messages", embed).await;
                                    }
                                }
                            }
                        }
                        serenity::FullEvent::GuildMemberAddition { new_member } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = new_member.guild_id.get() as i64;
                                let embed = serenity::CreateEmbed::new()
                                    .title("member joined")
                                    .field("user", new_member.user.mention().to_string(), false)
                                    .field("id", new_member.user.id.to_string(), false)
                                    .field("account created", format!("<t:{}:R>", new_member.user.created_at().timestamp()), false)
                                    .color(0x80F291)
                                    .thumbnail(new_member.user.face())
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "members", embed).await;
                            }
                        }
                        serenity::FullEvent::GuildMemberRemoval { guild_id, user, member_data_if_available: _ } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = guild_id.get() as i64;
                                let embed = serenity::CreateEmbed::new()
                                    .title("member left")
                                    .field("user", user.mention().to_string(), false)
                                    .field("id", user.id.to_string(), false)
                                    .color(0xF28080)
                                    .thumbnail(user.face())
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "members", embed).await;
                            }
                        }
                        serenity::FullEvent::GuildBanAddition { guild_id, banned_user } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = guild_id.get() as i64;
                                let embed = serenity::CreateEmbed::new()
                                    .title("member banned")
                                    .field("user", banned_user.mention().to_string(), false)
                                    .field("id", banned_user.id.to_string(), false)
                                    .color(0xF28080)
                                    .thumbnail(banned_user.face())
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "moderation", embed).await;
                            }
                        }
                        serenity::FullEvent::GuildBanRemoval { guild_id, unbanned_user } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = guild_id.get() as i64;
                                let embed = serenity::CreateEmbed::new()
                                    .title("member unbanned")
                                    .field("user", unbanned_user.mention().to_string(), false)
                                    .field("id", unbanned_user.id.to_string(), false)
                                    .color(0x80F291)
                                    .thumbnail(unbanned_user.face())
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "moderation", embed).await;
                            }
                        }
                        serenity::FullEvent::GuildRoleCreate { new } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = new.guild_id.get() as i64;
                                let embed = serenity::CreateEmbed::new()
                                    .title("role created")
                                    .field("role", new.mention().to_string(), false)
                                    .field("id", new.id.to_string(), false)
                                    .field("color", format!("#{:06x}", new.colour.0), false)
                                    .color(0x80F291)
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "roles", embed).await;
                            }
                        }
                        serenity::FullEvent::GuildRoleDelete { guild_id, removed_role_data_if_available, .. } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = guild_id.get() as i64;
                                let name = removed_role_data_if_available.as_ref().map(|r| r.name.clone()).unwrap_or_else(|| "unknown".to_string());
                                let embed = serenity::CreateEmbed::new()
                                    .title("role deleted")
                                    .field("role", name, false)
                                    .color(0xF28080)
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "roles", embed).await;
                            }
                        }
                        serenity::FullEvent::GuildRoleUpdate { old_data_if_available: _, new } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = new.guild_id.get() as i64;
                                let embed = serenity::CreateEmbed::new()
                                    .title("role updated")
                                    .field("role", new.mention().to_string(), false)
                                    .field("id", new.id.to_string(), false)
                                    .field("color", format!("#{:06x}", new.colour.0), false)
                                    .field("hoisted", new.hoist.to_string(), true)
                                    .field("mentionable", new.mentionable.to_string(), true)
                                    .color(0xF2D380)
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "roles", embed).await;
                            }
                        }
                        serenity::FullEvent::ChannelCreate { channel } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = channel.guild_id.get() as i64;
                                let kind = match channel.kind {
                                    serenity::ChannelType::Text => "text",
                                    serenity::ChannelType::Voice => "voice",
                                    serenity::ChannelType::Category => "category",
                                    serenity::ChannelType::News => "announcement",
                                    serenity::ChannelType::Stage => "stage",
                                    serenity::ChannelType::Forum => "forum",
                                    _ => "other",
                                };
                                let embed = serenity::CreateEmbed::new()
                                    .title("channel created")
                                    .field("channel", channel.mention().to_string(), false)
                                    .field("type", kind, true)
                                    .field("id", channel.id.to_string(), true)
                                    .color(0x80F291)
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "channels", embed).await;
                            }
                        }
                        serenity::FullEvent::ChannelDelete { channel, .. } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = channel.guild_id.get() as i64;
                                let kind = match channel.kind {
                                    serenity::ChannelType::Text => "text",
                                    serenity::ChannelType::Voice => "voice",
                                    serenity::ChannelType::Category => "category",
                                    serenity::ChannelType::News => "announcement",
                                    serenity::ChannelType::Stage => "stage",
                                    serenity::ChannelType::Forum => "forum",
                                    _ => "other",
                                };
                                let embed = serenity::CreateEmbed::new()
                                    .title("channel deleted")
                                    .field("channel", channel.name.clone(), false)
                                    .field("type", kind, true)
                                    .field("id", channel.id.to_string(), true)
                                    .color(0xF28080)
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "channels", embed).await;
                            }
                        }
                        serenity::FullEvent::ChannelUpdate { new, .. } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                let gid = new.guild_id.get() as i64;
                                let embed = serenity::CreateEmbed::new()
                                    .title("channel updated")
                                    .field("channel", new.mention().to_string(), false)
                                    .field("id", new.id.to_string(), true)
                                    .color(0xF2D380)
                                    .timestamp(chrono::Utc::now());
                                commands::auditlog::log_event(&ctx.http, &db, gid, "channels", embed).await;
                            }
                        }
                        serenity::FullEvent::VoiceStateUpdate { old, new } => {
                            if let Some(db) = ctx.data.read().await.get::<DbKey>().cloned() {
                                if let Some(gid) = new.guild_id {
                                    let user_id = new.user_id;
                                    let (action, channel_name) = if new.channel_id.is_some() {
                                        let ch = new.channel_id
                                            .map(|cid| format!("<#{}>", cid))
                                            .unwrap_or_else(|| "unknown".to_string());
                                        ("joined voice", ch)
                                    } else {
                                        let ch = old.as_ref()
                                            .and_then(|o| o.channel_id)
                                            .map(|cid| format!("<#{}>", cid))
                                            .unwrap_or_else(|| "unknown".to_string());
                                        ("left voice", ch)
                                    };
                                    let embed = serenity::CreateEmbed::new()
                                        .title("voice state update")
                                        .field("user", format!("<@{}>", user_id), false)
                                        .field("action", action, true)
                                        .field("channel", &channel_name, true)
                                        .color(0x5865F2)
                                        .timestamp(chrono::Utc::now());
                                    commands::auditlog::log_event(&ctx.http, &db, gid.get() as i64, "voice", embed).await;
                                }
                            }
                        }
                        serenity::FullEvent::InteractionCreate { interaction } => {
                            if let serenity::Interaction::Component(component) = interaction {
                                if component.data.custom_id.starts_with("help_") {
                                    let cogs = commands::utility::build_cogs(&framework.options().commands);
                                    let _ = commands::utility::handle_help_button(ctx, component, &cogs).await;
                                }
                                if component.data.custom_id.starts_with("poll_") {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        let _ = commands::fun::handle_poll_button(ctx, component, &db).await;
                                    }
                                }
                                if component.data.custom_id.starts_with("rr_") {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        let _ = commands::reactionrole::handle_setup_button(ctx, component, &db).await;
                                    }
                                }
                                if component.data.custom_id == "ticket_open" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::ticket::handle_ticket_open(ctx, component, &db).await {
                                            error!("ticket_open error: {:?}", e);
                                        }
                                    }
                                }
                                if component.data.custom_id == "ticket_close" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::ticket::handle_ticket_close(ctx, component, &db).await {
                                            error!("ticket_close error: {:?}", e);
                                        }
                                    }
                                }
                                if component.data.custom_id == "ticket_action_transcript" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::ticket::handle_ticket_transcript(ctx, component, &db).await {
                                            error!("ticket_transcript error: {:?}", e);
                                        }
                                    }
                                }
                                if component.data.custom_id == "ticket_action_archive" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::ticket::handle_ticket_archive(ctx, component, &db).await {
                                            error!("ticket_archive error: {:?}", e);
                                        }
                                    }
                                }
                                if component.data.custom_id == "ticket_action_delete" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::ticket::handle_ticket_delete(ctx, component, &db).await {
                                            error!("ticket_delete error: {:?}", e);
                                        }
                                    }
                                }
                                if component.data.custom_id == "auditlog_edit_config" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::auditlog::handle_auditlog_edit_config(ctx, component, &db).await {
                                            error!("auditlog_edit_config error: {:?}", e);
                                        }
                                    }
                                }
                            }
                            if let Some(modal) = interaction.clone().modal_submit() {
                                if modal.data.custom_id.starts_with("rr_modal_") {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        let _ = commands::reactionrole::handle_modal_submit(ctx, &modal, &db).await;
                                    }
                                }
                                if modal.data.custom_id == "ticket_modal" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::ticket::handle_ticket_modal(ctx, &modal, &db).await {
                                            error!("ticket_modal error: {:?}", e);
                                        }
                                    }
                                }
                                if modal.data.custom_id == "auditlog_modal" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        if let Err(e) = commands::auditlog::handle_auditlog_modal(ctx, &modal, &db).await {
                                            error!("auditlog_modal error: {:?}", e);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    Ok(())
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let guilds = ctx.cache.guilds();
                for guild_id in &guilds {
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        *guild_id,
                    )
                    .await?;
                    info!("Registered slash commands for guild {}", guild_id);
                }
                ctx.data.write().await.insert::<DbKey>(db.clone());
                commands::moderation::rotate_honeypots(&ctx.http, &db).await;
                Ok(Data {
                    _start_time: std::time::Instant::now(),
                    db,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    if let Err(why) = client.unwrap().start().await {
        error!("Client error: {:?}", why);
    }
}
