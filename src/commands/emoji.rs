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
    #[description = "emoji to steal (paste up to 20)"] emojis: String,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("this command can only be used in a server").await?;
            return Ok(());
        }
    };

    let parsed: Vec<(String, u64, bool)> = parse_all_emojis(&emojis);

    if parsed.is_empty() {
        ctx.say("couldn't parse any custom emojis. make sure you paste them directly.")
            .await?;
        return Ok(());
    }

    if parsed.len() > 20 {
        ctx.say("you can steal up to 20 emojis at a time").await?;
        return Ok(());
    }

    let existing = match guild_id.emojis(&ctx).await {
        Ok(e) => e,
        Err(e) => {
            error!("failed to fetch emojis: {:?}", e);
            ctx.say("failed to fetch server emojis").await?;
            return Ok(());
        }
    };

    let mut stolen = Vec::new();
    let mut failed = Vec::new();

    for (emoji_name, emoji_id, animated) in &parsed {
        let final_name = find_unique_name(emoji_name, &existing, &stolen);

        let extension = if *animated { "gif" } else { "png" };
        let image_url = format!("https://cdn.discordapp.com/emojis/{}.{}", emoji_id, extension);

        let response = match reqwest::get(&image_url).await {
            Ok(r) => match r.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    error!("failed to download emoji {} bytes: {:?}", emoji_name, e);
                    failed.push(emoji_name.clone());
                    continue;
                }
            },
            Err(e) => {
                error!("failed to download emoji {}: {:?}", emoji_name, e);
                failed.push(emoji_name.clone());
                continue;
            }
        };

        let mime = if *animated { "image/gif" } else { "image/png" };
        let b64 = format!(
            "data:{};base64,{}",
            mime,
            base64::engine::general_purpose::STANDARD.encode(&response)
        );

        match guild_id.create_emoji(&ctx, &final_name, &b64).await {
            Ok(e) => stolen.push((final_name, e)),
            Err(e) => {
                error!("failed to create emoji {}: {:?}", emoji_name, e);
                failed.push(emoji_name.clone());
            }
        }
    }

    let mut description = String::new();
    if !stolen.is_empty() {
        let mentions: Vec<String> = stolen.iter().map(|(name, e)| format!("`:{}` → {}", name, e)).collect();
        description.push_str(&format!("**stolen:**\n{}", mentions.join("\n")));
    }
    if !failed.is_empty() {
        if !description.is_empty() {
            description.push_str("\n\n");
        }
        description.push_str(&format!("**failed:** {}", failed.join(", ")));
    }

    let color = if failed.is_empty() { 0x80F291 } else { 0xF2D380 };
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("emoji steal")
                .description(&description)
                .color(color),
        ),
    )
    .await?;

    Ok(())
}

fn find_unique_name(base: &str, existing: &[serenity::Emoji], stolen: &[(String, serenity::Emoji)]) -> String {
    let taken: Vec<&str> = existing
        .iter()
        .map(|e| e.name.as_str())
        .chain(stolen.iter().map(|(name, _)| name.as_str()))
        .collect();

    if !taken.contains(&base) {
        return base.to_string();
    }

    let mut i = 2;
    loop {
        let candidate = format!("{}_{}", base, i);
        if !taken.contains(&candidate.as_str()) {
            return candidate;
        }
        i += 1;
    }
}

fn parse_all_emojis(input: &str) -> Vec<(String, u64, bool)> {
    let mut results = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        if remaining.starts_with("<a:") {
            if let Some(end) = remaining.find('>') {
                let raw = &remaining[..=end];
                let inner = &raw[3..raw.len() - 1];
                if let Some((name, id_str)) = inner.rsplit_once(':') {
                    if let Ok(id) = id_str.parse::<u64>() {
                        results.push((name.to_string(), id, true));
                    }
                }
                remaining = &remaining[end + 1..];
                continue;
            }
        } else if remaining.starts_with("<:") {
            if let Some(end) = remaining.find('>') {
                let raw = &remaining[..=end];
                let inner = &raw[2..raw.len() - 1];
                if let Some((name, id_str)) = inner.rsplit_once(':') {
                    if let Ok(id) = id_str.parse::<u64>() {
                        results.push((name.to_string(), id, false));
                    }
                }
                remaining = &remaining[end + 1..];
                continue;
            }
        }

        // skip one byte and continue scanning
        remaining = &remaining[1..];
    }

    results
}
