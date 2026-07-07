mod commands;

use poise::serenity_prelude as serenity;
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info};

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
            ],
            on_error: |error| {
                Box::pin(async move {
                    if let poise::FrameworkError::Command { error, ctx, .. } = error {
                        error!("Command error: {:?}", error);
                        let _ = ctx
                            .say(format!("command failed: {:?}", error))
                            .await;
                    }
                })
            },
            event_handler: |ctx, event, _framework, _data| {
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
                        serenity::FullEvent::InteractionCreate { interaction } => {
                            if let serenity::Interaction::Component(component) = interaction {
                                if component.data.custom_id.starts_with("help_") {
                                    let _ = commands::utility::handle_help_button(ctx, component).await;
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
                                        let _ = commands::ticket::handle_ticket_open(ctx, component, &db).await;
                                    }
                                }
                                if component.data.custom_id == "ticket_close" {
                                    let db = ctx.data.read().await.get::<DbKey>().cloned();
                                    if let Some(db) = db {
                                        let _ = commands::ticket::handle_ticket_close(ctx, component, &db).await;
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
                                        let _ = commands::ticket::handle_ticket_modal(ctx, &modal, &db).await;
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
