use base64::Engine;
use poise::serenity_prelude as serenity;
use tracing::error;

use crate::{Context, Error};

#[poise::command(
    slash_command,
    category = "utility",
    required_permissions = "MANAGE_GUILD_EXPRESSIONS"
)]
pub async fn steal(
    ctx: Context<'_>,
    #[description = "emoji to steal"] emoji: serenity::Emoji,
    #[description = "name for the emoji (optional)"] name: Option<String>,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("this command can only be used in a server").await?;
            return Ok(());
        }
    };

    let final_name = match name {
        Some(n) => n,
        None => emoji.name.clone(),
    };

    let existing = match guild_id.emojis(&ctx).await {
        Ok(e) => e,
        Err(e) => {
            error!("failed to fetch emojis: {:?}", e);
            ctx.say("failed to fetch server emojis").await?;
            return Ok(());
        }
    };

    let unique_name = if existing.iter().any(|e| e.name == final_name) {
        let mut candidate = format!("{}_2", final_name);
        let mut i = 3;
        while existing.iter().any(|e| e.name == candidate) {
            candidate = format!("{}_{}", final_name, i);
            i += 1;
        }
        candidate
    } else {
        final_name
    };

    let image_url = emoji.url();
    let response = match reqwest::get(&image_url).await {
        Ok(r) => match r.bytes().await {
            Ok(b) => b,
            Err(e) => {
                error!("failed to download emoji bytes: {:?}", e);
                ctx.say("failed to download emoji image").await?;
                return Ok(());
            }
        },
        Err(e) => {
            error!("failed to download emoji: {:?}", e);
            ctx.say("failed to download emoji").await?;
            return Ok(());
        }
    };

    let mime = if emoji.animated { "image/gif" } else { "image/png" };
    let b64 = format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(&response)
    );

    let new_emoji = match guild_id.create_emoji(&ctx, &unique_name, &b64).await {
        Ok(e) => e,
        Err(e) => {
            error!("failed to create emoji: {:?}", e);
            ctx.say(format!("failed to create emoji: {}", e)).await?;
            return Ok(());
        }
    };

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("emoji stolen")
                .description(format!("added {} as {}", emoji, new_emoji))
                .color(0x80F291),
        ),
    )
    .await?;

    Ok(())
}
