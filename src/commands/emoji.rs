use base64::Engine;
use poise::serenity_prelude as serenity;

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
    let guild_id = ctx.guild_id().ok_or("this command can only be used in a server")?;

    let final_name = match name {
        Some(n) => n,
        None => emoji.name.clone(),
    };

    let existing = guild_id
        .emojis(&ctx)
        .await
        .unwrap_or_default();

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
    let response = reqwest::get(&image_url).await?.bytes().await?;
    let b64 = format!(
        "data:{};base64,{}",
        if emoji.animated { "image/gif" } else { "image/png" },
        base64::engine::general_purpose::STANDARD.encode(&response)
    );

    let new_emoji = guild_id.create_emoji(&ctx, &unique_name, &b64).await?;

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
