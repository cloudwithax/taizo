use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;
use rand::Rng;

async fn ensure_account(db: &sqlx::PgPool, user_id: u64) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM economy WHERE user_id = $1)")
        .bind(user_id as i64)
        .fetch_one(db)
        .await?;
    Ok(exists)
}

async fn get_balance(db: &sqlx::PgPool, user_id: u64) -> Result<(i64, i64, i64), sqlx::Error> {
    let row = sqlx::query_as::<_, (i64, i64, i64)>("SELECT wallet, bank, worth FROM economy WHERE user_id = $1")
        .bind(user_id as i64)
        .fetch_one(db)
        .await?;
    Ok(row)
}

async fn update_worth(db: &sqlx::PgPool, user_id: u64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE economy SET worth = wallet + bank WHERE user_id = $1")
        .bind(user_id as i64)
        .execute(db)
        .await?;
    Ok(())
}

async fn add_money(db: &sqlx::PgPool, user_id: u64, amount: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE economy SET wallet = wallet + $1 WHERE user_id = $2")
        .bind(amount)
        .bind(user_id as i64)
        .execute(db)
        .await?;
    update_worth(db, user_id).await?;
    Ok(())
}

async fn remove_money(db: &sqlx::PgPool, user_id: u64, amount: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE economy SET wallet = wallet - $1 WHERE user_id = $2")
        .bind(amount)
        .bind(user_id as i64)
        .execute(db)
        .await?;
    update_worth(db, user_id).await?;
    Ok(())
}

/// opens a bank account for you to use
#[poise::command(slash_command, aliases("oacc"))]
pub async fn openaccount(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you already have an account!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("INSERT INTO economy (user_id, wallet, bank, worth) VALUES ($1, 0, 0, 0)")
        .bind(user_id as i64)
        .execute(&ctx.data().db)
        .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description("✅ account created!")
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// closes your bank account
#[poise::command(slash_command, aliases("cacc"))]
pub async fn closeaccount(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you don't have an account!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("DELETE FROM economy WHERE user_id = $1")
        .bind(user_id as i64)
        .execute(&ctx.data().db)
        .await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description("✅ account deleted.")
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// check your balance
#[poise::command(slash_command, aliases("bal"))]
pub async fn balance(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (wallet, bank, _worth) = get_balance(&ctx.data().db, user_id).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("balance")
                .field("wallet", format!("${}", wallet), true)
                .field("bank", format!("${}", bank), true)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// work and receive a random amount of money
#[poise::command(slash_command)]
pub async fn work(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let amount = rand::thread_rng().gen_range(50..=500);
    add_money(&ctx.data().db, user_id, amount).await?;

    let responses = [
        format!("nice work {}, you got **${}** for your good work today.", ctx.author().mention(), amount),
        format!("great work {}, you received **${}**.", ctx.author().mention(), amount),
        format!("keep up the good work {}, you got **${}**", ctx.author().mention(), amount),
        format!("phenomenal work {}, you received **${}**.", ctx.author().mention(), amount),
    ];

    let msg = responses[rand::thread_rng().gen_range(0..responses.len())].clone();

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("work")
                .description(&msg)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// work as a slut and receive or lose a random amount of money
#[poise::command(slash_command)]
pub async fn slut(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let outcome = rand::thread_rng().gen_range(0..2);
    let amount = rand::thread_rng().gen_range(50..=500);

    if outcome == 0 {
        add_money(&ctx.data().db, user_id, amount).await?;
        let responses = [
            format!("you got frisky with a bum off the street, he paid you **${}**", amount),
            format!("you did a little quickie with a handsome hunk, he paid you **${}**", amount),
            format!("you did a routine pole dance and got paid **${}**", amount),
            format!("you did a lap dance and got tipped **${}**", amount),
            format!("you played dirty with a business CEO and he paid you **${}**", amount),
        ];
        let msg = responses[rand::thread_rng().gen_range(0..responses.len())].clone();
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .title("slut")
                    .description(&msg)
                    .color(0x80F291),
            ),
        )
        .await?;
    } else {
        remove_money(&ctx.data().db, user_id, amount).await?;
        let responses = [
            format!("you got caught doing it out in public and the police fined you **${}**", amount),
            format!("you got frisky with a dude and he didn't like it, you refunded him **${}**", amount),
            format!("a dude you had sex with stole **${}** from you!", amount),
            format!("you got beat up by another hooker and she stole **${}** from you!", amount),
            format!("your job caught you slacking on the job and took **${}** from you!", amount),
            format!("you tripped on the sidewalk and lost **${}**", amount),
        ];
        let msg = responses[rand::thread_rng().gen_range(0..responses.len())].clone();
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .title("slut")
                    .description(&msg)
                    .color(0xF28080),
            ),
        )
        .await?;
    }

    Ok(())
}

/// commit a crime and receive or lose a random amount of money
#[poise::command(slash_command)]
pub async fn crime(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let outcome = rand::thread_rng().gen_range(0..2);
    let amount = rand::thread_rng().gen_range(50..=500);

    if outcome == 0 {
        add_money(&ctx.data().db, user_id, amount).await?;
        let responses = [
            format!("you robbed a jewelry store and got away with it. you received **${}**", amount),
            format!("you stuck up a gas station clerk and stole **${}**", amount),
            format!("you mugged a citizen and stole **${}**", amount),
            format!("you broke into a house and got away with pricey electronics. you sold them and got a total of **${}**", amount),
            format!("you broke into a car and drove off with it. you got **${}** for selling it.", amount),
            format!("you sold drugs and got **${}**", amount),
            format!("you delivered a suspicious package and earned **${}**", amount),
        ];
        let msg = responses[rand::thread_rng().gen_range(0..responses.len())].clone();
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .title("crime")
                    .description(&msg)
                    .color(0x80F291),
            ),
        )
        .await?;
    } else {
        remove_money(&ctx.data().db, user_id, amount).await?;
        let responses = [
            format!("you attempted to mug a citizen, but got caught! you were fined **${}**", amount),
            format!("you broke into a pawn shop and you got caught! you were fined **${}**", amount),
            format!("while robbing a store, you dropped your wallet! you lost **${}**", amount),
            format!("you stole money from a supermarket cashier and someone stole it back. you lost **${}**", amount),
            format!("your car got pulled over for speeding whilst heading to a heist. you got fined **${}**", amount),
            format!("you stole poker chips from a casino and got fined **${}**", amount),
        ];
        let msg = responses[rand::thread_rng().gen_range(0..responses.len())].clone();
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .title("crime")
                    .description(&msg)
                    .color(0xF28080),
            ),
        )
        .await?;
    }

    Ok(())
}

/// get a daily bonus of $500
#[poise::command(slash_command)]
pub async fn daily(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    add_money(&ctx.data().db, user_id, 500).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("daily")
                .description("you claimed your daily bonus and received **$500**!")
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// get a weekly bonus of $5,000
#[poise::command(slash_command)]
pub async fn weekly(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    add_money(&ctx.data().db, user_id, 5000).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("weekly")
                .description("you claimed your weekly bonus and received **$5,000**!")
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}

/// deposit money from your wallet to your bank
#[poise::command(slash_command, aliases("dep"))]
pub async fn deposit(
    ctx: Context<'_>,
    #[description = "amount to deposit"] amount: i64,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (wallet, _bank, _worth) = get_balance(&ctx.data().db, user_id).await?;

    if amount <= 0 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 the amount must be at least **$1**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if wallet < amount {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you don't have that amount in your wallet!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("UPDATE economy SET wallet = wallet - $1, bank = bank + $1 WHERE user_id = $2")
        .bind(amount)
        .bind(user_id as i64)
        .execute(&ctx.data().db)
        .await?;
    update_worth(&ctx.data().db, user_id).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ deposited **${}** to your bank.", amount))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// deposit all money from your wallet to your bank
#[poise::command(slash_command)]
pub async fn depositall(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (wallet, _bank, _worth) = get_balance(&ctx.data().db, user_id).await?;

    if wallet <= 0 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you have nothing to deposit!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("UPDATE economy SET wallet = 0, bank = bank + $1 WHERE user_id = $2")
        .bind(wallet)
        .bind(user_id as i64)
        .execute(&ctx.data().db)
        .await?;
    update_worth(&ctx.data().db, user_id).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ deposited **${}** to your bank.", wallet))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// withdraw money from your bank to your wallet
#[poise::command(slash_command, aliases("with"))]
pub async fn withdraw(
    ctx: Context<'_>,
    #[description = "amount to withdraw"] amount: i64,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (_wallet, bank, _worth) = get_balance(&ctx.data().db, user_id).await?;

    if amount <= 0 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 the amount must be at least **$1**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if bank < amount {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you don't have that amount in your bank!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("UPDATE economy SET bank = bank - $1, wallet = wallet + $1 WHERE user_id = $2")
        .bind(amount)
        .bind(user_id as i64)
        .execute(&ctx.data().db)
        .await?;
    update_worth(&ctx.data().db, user_id).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ withdrew **${}** to your wallet.", amount))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// withdraw all money from your bank to your wallet
#[poise::command(slash_command)]
pub async fn withdrawall(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (_wallet, bank, _worth) = get_balance(&ctx.data().db, user_id).await?;

    if bank <= 0 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you have nothing to withdraw!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("UPDATE economy SET bank = 0, wallet = wallet + $1 WHERE user_id = $2")
        .bind(bank)
        .bind(user_id as i64)
        .execute(&ctx.data().db)
        .await?;
    update_worth(&ctx.data().db, user_id).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ withdrew **${}** to your wallet.", bank))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// pay someone using your wallet's balance
#[poise::command(slash_command)]
pub async fn pay(
    ctx: Context<'_>,
    #[description = "user to pay"] user: serenity::Member,
    #[description = "amount to pay"] amount: i64,
) -> Result<(), Error> {
    let sender_id = ctx.author().id.get();
    let receiver_id = user.user.id.get();

    if sender_id == receiver_id {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you can't pay yourself!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if amount <= 0 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 the amount must be at least **$1**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let sender_exists = ensure_account(&ctx.data().db, sender_id).await?;
    if !sender_exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let receiver_exists = ensure_account(&ctx.data().db, receiver_id).await?;
    if !receiver_exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!("🛑 {} needs to open an account first!", user.mention()))
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (wallet, _bank, _worth) = get_balance(&ctx.data().db, sender_id).await?;
    if wallet < amount {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you don't have that amount in your wallet!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    sqlx::query("UPDATE economy SET wallet = wallet - $1 WHERE user_id = $2")
        .bind(amount)
        .bind(sender_id as i64)
        .execute(&ctx.data().db)
        .await?;
    sqlx::query("UPDATE economy SET wallet = wallet + $1 WHERE user_id = $2")
        .bind(amount)
        .bind(receiver_id as i64)
        .execute(&ctx.data().db)
        .await?;
    update_worth(&ctx.data().db, sender_id).await?;
    update_worth(&ctx.data().db, receiver_id).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .description(format!("✅ successfully sent **${}** to {}", amount, user.mention()))
                .color(0x80F291),
        ),
    )
    .await?;
    Ok(())
}

/// flip a coin and land on your side to win 2x the betted amount
#[poise::command(slash_command, aliases("flip"))]
pub async fn coinflip(
    ctx: Context<'_>,
    #[description = "amount to bet"] amount: i64,
    #[description = "heads or tails"] side: String,
) -> Result<(), Error> {
    let side_lower = side.to_lowercase();
    if side_lower != "heads" && side_lower != "tails" {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 invalid side! you must choose either **heads** or **tails**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (wallet, _bank, _worth) = get_balance(&ctx.data().db, user_id).await?;

    if amount < 50 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 the minimum bet is **$50**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if amount > 10000 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 the maximum bet is **$10,000**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if wallet < amount {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you don't have that amount in your wallet!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let flip = if rand::thread_rng().gen_range(0..2) == 0 { "heads" } else { "tails" };

    if side_lower == flip {
        add_money(&ctx.data().db, user_id, amount).await?;
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!("📀 {} flipped **{}** and won **${}**", ctx.author().mention(), flip, amount))
                    .color(0x80F291),
            ),
        )
        .await?;
    } else {
        remove_money(&ctx.data().db, user_id, amount).await?;
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!("💿 {} flipped **{}** and lost **${}**", ctx.author().mention(), flip, amount))
                    .color(0xF28080),
            ),
        )
        .await?;
    }

    Ok(())
}

/// guess if the missing number will be higher or lower and win double or lose
#[poise::command(slash_command, aliases("hilo", "highlo", "hilow"))]
pub async fn highlow(
    ctx: Context<'_>,
    #[description = "amount to bet"] amount: i64,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let exists = ensure_account(&ctx.data().db, user_id).await?;

    if !exists {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you need to open an account first!\n`/openaccount`")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (wallet, _bank, _worth) = get_balance(&ctx.data().db, user_id).await?;

    if amount < 50 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 the minimum bet is **$50**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if amount > 10000 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 the maximum bet is **$10,000**!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if wallet < amount {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("🛑 you don't have that amount in your wallet!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (hi, low) = {
        let mut rng = rand::thread_rng();
        (rng.gen_range(1..=25), rng.gen_range(1..=25))
    };

    let buttons = vec![
        serenity::CreateButton::new("highlow_up")
            .label("⬆ higher")
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new("highlow_down")
            .label("⬇ lower")
            .style(serenity::ButtonStyle::Danger),
    ];

    let msg = ctx.send(
        poise::CreateReply::default()
            .embed(
                serenity::CreateEmbed::new()
                    .title("hi-lo")
                    .description(format!(
                        "**test your luck, {}!**\nis the missing number higher or lower?\n```1) {}\n2) ??```",
                        ctx.author().mention(), hi
                    ))
                    .color(0xF28080),
            )
            .components(vec![serenity::CreateActionRow::Buttons(buttons)]),
    )
    .await?;

    let reply_msg = msg.into_message().await?;

    use poise::futures_util::StreamExt;

    let interaction = serenity::collector::ComponentInteractionCollector::new(&ctx.serenity_context().shard)
        .message_id(reply_msg.id)
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(30))
        .stream()
        .next()
        .await;

    match interaction {
        Some(interaction) => {
            let guessed_up = interaction.data.custom_id == "highlow_up";

            let won = (guessed_up && hi >= low) || (!guessed_up && hi <= low);

            if won {
                add_money(&ctx.data().db, user_id, amount).await?;
            } else {
                remove_money(&ctx.data().db, user_id, amount).await?;
            }

            let (description, color) = if won {
                (
                    format!("✅ {} guessed correctly and won **${}**!", ctx.author().mention(), amount),
                    0x80F291,
                )
            } else {
                (
                    format!("❌ {} guessed incorrectly and lost **${}**", ctx.author().mention(), amount),
                    0xF28080,
                )
            };

            interaction
                .create_response(
                    &ctx,
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new()
                            .embed(
                                serenity::CreateEmbed::new()
                                    .description(description)
                                    .color(color),
                            )
                            .components(vec![]),
                    ),
                )
                .await?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default().embed(
                    serenity::CreateEmbed::new()
                        .description("❌ you didn't answer in time. the bet was cancelled.")
                        .color(0xF28080),
                ),
            )
            .await?;
        }
    }

    Ok(())
}

/// play a hand of blackjack
#[poise::command(slash_command, aliases("bj"))]
pub async fn blackjack(
    ctx: Context<'_>,
    #[description = "amount to bet"] mut bet: i64,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let db = &ctx.data().db;

    if !ensure_account(db, user_id).await? {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("you need to open an account first!")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    if bet < 50 || bet > 10_000 {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("bet must be between $50 and $10,000.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let (wallet, _, _) = get_balance(db, user_id).await?;
    if wallet < bet {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description(format!("you only have ${} in your wallet.", wallet))
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    fn random_card() -> (String, u8) {
        let mut rng = rand::thread_rng();
        let rank: u8 = rng.gen_range(0..13);
        let suit: u8 = rng.gen_range(0..4);
        let suit_str = match suit {
            0 => "♠",
            1 => "♥",
            2 => "♦",
            _ => "♣",
        };
        match rank {
            0 => (format!("A{}", suit_str), 11),
            1 => (format!("2{}", suit_str), 2),
            2 => (format!("3{}", suit_str), 3),
            3 => (format!("4{}", suit_str), 4),
            4 => (format!("5{}", suit_str), 5),
            5 => (format!("6{}", suit_str), 6),
            6 => (format!("7{}", suit_str), 7),
            7 => (format!("8{}", suit_str), 8),
            8 => (format!("9{}", suit_str), 9),
            9 => (format!("10{}", suit_str), 10),
            10 => (format!("J{}", suit_str), 10),
            11 => (format!("Q{}", suit_str), 10),
            _ => (format!("K{}", suit_str), 10),
        }
    }

    fn hand_value(hand: &[(String, u8)]) -> u8 {
        let mut total: u8 = hand.iter().map(|(_, v)| v).sum();
        let aces = hand.iter().filter(|(r, _)| r.starts_with('A')).count();
        for _ in 0..aces {
            if total > 21 {
                total -= 10;
            }
        }
        total
    }

    fn hand_display(hand: &[(String, u8)]) -> String {
        hand.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>().join(" ")
    }

    let player_card1 = random_card();
    let player_card2 = random_card();
    let dealer_card1 = random_card();
    let dealer_card2 = random_card();

    let mut player_hand = vec![player_card1, player_card2];
    let mut dealer_hand = vec![dealer_card1, dealer_card2];

    let player_total = hand_value(&player_hand);

    fn game_embed(
        player_hand: &[(String, u8)],
        dealer_hand: &[(String, u8)],
        hide_dealer: bool,
        title: &str,
        color: u32,
    ) -> serenity::CreateEmbed {
        let dealer_str = if hide_dealer {
            format!("{} ??", dealer_hand[0].0)
        } else {
            hand_display(dealer_hand)
        };
        let player_str = hand_display(player_hand);
        let dealer_val = if hide_dealer {
            format!("{}", dealer_hand[0].1)
        } else {
            format!("{}", hand_value(dealer_hand))
        };
        serenity::CreateEmbed::new()
            .title(title)
            .field("dealer", format!("{}\n({})", dealer_str, dealer_val), false)
            .field("player", format!("{}\n({})", player_str, hand_value(player_hand)), false)
            .color(color)
    }

    let initial_embed = game_embed(&player_hand, &dealer_hand, true, "blackjack", 0xF2D380);

    if player_total == 21 {
        if hand_value(&dealer_hand) == 21 {
            let result_embed = game_embed(&player_hand, &dealer_hand, false, "push — both have blackjack", 0xF2D380);
            ctx.send(poise::CreateReply::default().embed(result_embed)).await?;
            return Ok(());
        }
        remove_money(db, user_id, bet).await?;
        let payout = (bet as f64 * 1.5) as i64;
        add_money(db, user_id, bet + payout).await?;
        let result_embed = game_embed(&player_hand, &dealer_hand, false, "blackjack! you win!", 0x80F291)
            .field("payout", format!("+${}", payout), false);
        ctx.send(poise::CreateReply::default().embed(result_embed)).await?;
        return Ok(());
    }

    if hand_value(&dealer_hand) == 21 {
        remove_money(db, user_id, bet).await?;
        let result_embed = game_embed(&player_hand, &dealer_hand, false, "dealer has blackjack. you lose.", 0xF28080)
            .field("loss", format!("-${}", bet), false);
        ctx.send(poise::CreateReply::default().embed(result_embed)).await?;
        return Ok(());
    }

    let buttons = vec![
        serenity::CreateButton::new("bj_hit")
            .label("hit")
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new("bj_stand")
            .label("stand")
            .style(serenity::ButtonStyle::Danger),
        serenity::CreateButton::new("bj_double")
            .label("double down")
            .style(serenity::ButtonStyle::Primary)
            .disabled(wallet < bet * 2),
    ];

    let msg = ctx.send(
        poise::CreateReply::default()
            .embed(initial_embed)
            .components(vec![serenity::CreateActionRow::Buttons(buttons)]),
    )
    .await?;

    let reply_msg = msg.into_message().await?;

    loop {
        use poise::futures_util::StreamExt;

        let interaction = serenity::collector::ComponentInteractionCollector::new(
            &ctx.serenity_context().shard,
        )
        .message_id(reply_msg.id)
        .author_id(ctx.author().id)
        .timeout(std::time::Duration::from_secs(30))
        .stream()
        .next()
        .await;

        match interaction {
            Some(interaction) => {
                let action = interaction.data.custom_id.clone();

                if action == "bj_hit" {
                    let new_card = random_card();
                    player_hand.push(new_card);
                    let pt = hand_value(&player_hand);

                    if pt > 21 {
                        remove_money(db, user_id, bet).await?;
                        let result_embed = game_embed(&player_hand, &dealer_hand, false, "bust! you lose.", 0xF28080)
                            .field("loss", format!("-${}", bet), false);
                        interaction.create_response(
                            &ctx,
                            serenity::CreateInteractionResponse::UpdateMessage(
                                serenity::CreateInteractionResponseMessage::new()
                                    .embed(result_embed)
                                    .components(vec![]),
                            ),
                        ).await?;
                        return Ok(());
                    }

                    let updated_embed = game_embed(&player_hand, &dealer_hand, true, "blackjack", 0xF2D380);
                    interaction.create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::new()
                                .embed(updated_embed)
                                .components(vec![
                                    serenity::CreateActionRow::Buttons(vec![
                                        serenity::CreateButton::new("bj_hit")
                                            .label("hit")
                                            .style(serenity::ButtonStyle::Success),
                                        serenity::CreateButton::new("bj_stand")
                                            .label("stand")
                                            .style(serenity::ButtonStyle::Danger),
                                    ]),
                                ]),
                        ),
                    ).await?;
                } else if action == "bj_stand" {
                    while hand_value(&dealer_hand) < 17 {
                        dealer_hand.push(random_card());
                    }
                    let dt = hand_value(&dealer_hand);
                    let pt = hand_value(&player_hand);

                    let (title, color, payout) = if dt > 21 {
                        ("dealer busts! you win!", 0x80F291, bet)
                    } else if pt > dt {
                        ("you win!", 0x80F291, bet)
                    } else if pt < dt {
                        ("dealer wins.", 0xF28080, -bet)
                    } else {
                        ("push.", 0xF2D380, 0)
                    };

                    if payout > 0 {
                        add_money(db, user_id, payout).await?;
                    } else if payout < 0 {
                        remove_money(db, user_id, -payout).await?;
                    }

                    let result_embed = game_embed(&player_hand, &dealer_hand, false, title, color)
                        .field(if payout >= 0 { "winnings" } else { "loss" }, format!("{}${}", if payout >= 0 { "+" } else { "" }, payout.abs()), false);
                    interaction.create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::new()
                                .embed(result_embed)
                                .components(vec![]),
                        ),
                    ).await?;
                    return Ok(());
                } else if action == "bj_double" {
                    if wallet < bet * 2 {
                        interaction.create_response(
                            &ctx,
                            serenity::CreateInteractionResponse::Message(
                                serenity::CreateInteractionResponseMessage::new()
                                    .content("you can't afford to double down!")
                                    .ephemeral(true),
                            ),
                        ).await?;
                        continue;
                    }
                    bet *= 2;
                    let new_card = random_card();
                    player_hand.push(new_card);
                    let pt = hand_value(&player_hand);

                    if pt > 21 {
                        remove_money(db, user_id, bet).await?;
                        let result_embed = game_embed(&player_hand, &dealer_hand, false, "bust! you lose.", 0xF28080)
                            .field("loss", format!("-${}", bet), false);
                        interaction.create_response(
                            &ctx,
                            serenity::CreateInteractionResponse::UpdateMessage(
                                serenity::CreateInteractionResponseMessage::new()
                                    .embed(result_embed)
                                    .components(vec![]),
                            ),
                        ).await?;
                        return Ok(());
                    }

                    while hand_value(&dealer_hand) < 17 {
                        dealer_hand.push(random_card());
                    }
                    let dt = hand_value(&dealer_hand);

                    let (title, color, payout) = if dt > 21 {
                        ("dealer busts! you win!", 0x80F291, bet)
                    } else if pt > dt {
                        ("you win!", 0x80F291, bet)
                    } else if pt < dt {
                        ("dealer wins.", 0xF28080, -bet)
                    } else {
                        ("push.", 0xF2D380, 0)
                    };

                    if payout > 0 {
                        add_money(db, user_id, payout).await?;
                    } else if payout < 0 {
                        remove_money(db, user_id, -payout).await?;
                    }

                    let result_embed = game_embed(&player_hand, &dealer_hand, false, title, color)
                        .field(if payout >= 0 { "winnings" } else { "loss" }, format!("{}${}", if payout >= 0 { "+" } else { "" }, payout.abs()), false);
                    interaction.create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::new()
                                .embed(result_embed)
                                .components(vec![]),
                        ),
                    ).await?;
                    return Ok(());
                }
            }
            None => {
                ctx.send(
                    poise::CreateReply::default().embed(
                        serenity::CreateEmbed::new()
                            .description("blackjack timed out.")
                            .color(0xF28080),
                    ),
                )
                .await?;
                return Ok(());
            }
        }
    }
}

/// shows the global money leaderboard
#[poise::command(slash_command, aliases("lboard", "lb"))]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let rows = sqlx::query_as::<_, (i64, i64)>("SELECT user_id, worth FROM economy WHERE worth > 0 ORDER BY worth DESC LIMIT 10")
        .fetch_all(&ctx.data().db)
        .await?;

    if rows.is_empty() {
        ctx.send(
            poise::CreateReply::default().embed(
                serenity::CreateEmbed::new()
                    .description("no entries yet.")
                    .color(0xF28080),
            ),
        )
        .await?;
        return Ok(());
    }

    let mut description = String::new();
    for (i, (user_id, worth)) in rows.iter().enumerate() {
        description.push_str(&format!("{}. <@{}> — **${}**\n", i + 1, user_id, worth));
    }

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title("leaderboard")
                .description(&description)
                .color(0xF28080),
        ),
    )
    .await?;
    Ok(())
}
