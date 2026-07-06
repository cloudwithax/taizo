use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// shows more info about the bot
#[poise::command(slash_command, aliases("ab"))]
pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let uptime = data._start_time.elapsed();
    let uptime_str = format_uptime(uptime);

    let guild_count = ctx.cache().guilds().len();
    let current_user = ctx.cache().current_user().clone();

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("about")
                .thumbnail(current_user.face())
                .field("servers", guild_count.to_string(), true)
                .field("owner", "<@!313314995687391234>", true)
                .field("commands", "see /help", true)
                .field("uptime", &uptime_str, true)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// shows how long the bot has been up
#[poise::command(slash_command)]
pub async fn uptime(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    let uptime = data._start_time.elapsed();
    let uptime_str = format_uptime(uptime);

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(&uptime_str)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// sends the bot's invite link
#[poise::command(slash_command, aliases("inv"))]
pub async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("invite me")
                .url("https://discord.com/oauth2/authorize")
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// sends the bot's privacy policy
#[poise::command(slash_command, aliases("priv"))]
pub async fn privacy(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("privacy policy")
                .description("taizo does not record, store, or log any text channel conversations or voice channel conversations. The only data we store is your server ID to provide functionality for custom features. If you have any concerns, please reach out.")
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// vote for taizo
#[poise::command(slash_command)]
pub async fn vote(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("vote")
                .url("https://top.gg")
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// join the support server
#[poise::command(slash_command)]
pub async fn support(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("join the support server")
                .url("https://discord.gg/NwhwCPS")
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

fn format_uptime(uptime: std::time::Duration) -> String {
    let secs = uptime.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{}d", days));
    }
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    parts.push(format!("{}s", seconds));

    parts.join(" ")
}
