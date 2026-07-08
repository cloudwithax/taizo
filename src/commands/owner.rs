use crate::{Context, Error};
use std::process::Command;

const SERVICE_NAME: &str = "taizo.service";

fn is_owner(ctx: Context<'_>) -> bool {
    let owner_id = std::env::var("OWNER_ID").expect("missing OWNER_ID");
    ctx.author().id.to_string() == owner_id
}

/// restarts the bot service (owner only)
#[poise::command(slash_command, category = "owner")]
pub async fn restart(ctx: Context<'_>) -> Result<(), Error> {
    if !is_owner(ctx) {
        ctx.say("owner only").await?;
        return Ok(());
    }

    ctx.say("restarting...").await?;

    let output = Command::new("sudo")
        .args(["systemctl", "restart", SERVICE_NAME])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            ctx.say("restarted").await?;
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            ctx.say(format!("restart failed: {}", stderr.trim())).await?;
        }
        Err(e) => {
            ctx.say(format!("failed to run systemctl: {}", e)).await?;
        }
    }

    Ok(())
}

/// stops the bot service (owner only)
#[poise::command(slash_command, category = "owner")]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    if !is_owner(ctx) {
        ctx.say("owner only").await?;
        return Ok(());
    }

    ctx.say("stopping...").await?;

    let output = Command::new("sudo")
        .args(["systemctl", "stop", SERVICE_NAME])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            ctx.say("stopped").await?;
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            ctx.say(format!("stop failed: {}", stderr.trim())).await?;
        }
        Err(e) => {
            ctx.say(format!("failed to run systemctl: {}", e)).await?;
        }
    }

    Ok(())
}
