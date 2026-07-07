# AGENTS.md

## Project

Rust Discord bot (serenity 0.12 + poise 0.6) with PostgreSQL (sqlx). Single crate, no workspace.

## Rules

- All visible copy (embed titles, descriptions, field names, field values) must be strictly lowercase. No exceptions.
- Please reference #
## Build and run

```bash
cargo build
cargo run
```

## When you make changes
- Use `cargo clippy && cargo check && cargo build` to verify changes
- Note any pitfalls you experienced in the library/modules in AGENTS.md
- Commit changes
- Push changes to feature branch 
- Merges to main will happen at the developers discretion


## Schema

`schema.sql` is embedded via `include_str!` and auto-applied on every startup (split by `;`). No migrations tool — edits to `schema.sql` take effect next boot.

## Environment

Requires `.env` with `TOKEN` and `DATABASE_URL` (loaded by dotenvy).

## Commands

Register in `main.rs` under `framework.options().commands`. Each module lives in `src/commands/`:
- `economy.rs` — bank accounts, balance, work/crime/slut, gambling, leaderboard
- `fun.rs` — say, choose, hug, kiss, embed, poll, snipe, reddit, owoify, etc.
- `info.rs` — about, uptime, invite, privacy, vote, support
- `moderation.rs` — ban, kick, mute, warn, purge, setwelcome/setleave
- `utility.rs` — help (paginated buttons), ping, serverinfo, userinfo, avatar, whois

## Conventions

- Commands use poise's `#[poise::command]` macro with `slash_command` and `prefix_command`.
- DB access: pass `sqlx::PgPool` via `ctx.data.read().await`. User IDs are stored as `i64` (cast from `u64`).
- Error type is `Box<dyn std::error::Error + Send + Sync>`. Commands return `Result<(), Error>`.
- Event handlers for message logging and poll button interactions are in `main.rs` `event_handler`.
- Subcommands: poise 0.6 uses the **function name** as the slash command name. There is no `name` param in the macro to override it. Name functions accordingly (e.g. `create` not `honeypot_create`). Reference them in the parent's `subcommands("create", "remove")`.

## Serenity/Poise Pitfalls

- **Subcommand naming**: poise 0.6 has no `name` attribute in `#[poise::command]`. The function name IS the slash command name. `honeypot_create` becomes `/honeypot honeypot_create`. Fix: name functions just `create`, `remove`, etc.
- **Reactions are unreliable**: Discord heavily limits reaction-based interactions for bots. Use button components (`CreateButton` + `CreateActionRow`) instead of `msg.react()`.
- **Button interactions must be acknowledged**: If you add buttons but don't handle `InteractionCreate`, users see "interaction failed". Every button custom_id needs a handler in the event handler that calls `create_response` (even just `UpdateMessage` with empty components).
- **`GuildChannel.id` is a field, not a method**: serenity 0.12 — use `channel.id.get()` not `channel.id().get()`.
- **`CreateChannel` API**: `guild_id.create_channel(&ctx, CreateChannel::new("name").kind(ChannelType::Text)).await` — first arg is `&impl CacheHttp` (ctx works).
- **ComponentInteractionCollector lifetime**: The collector borrows from `ctx.serenity_context().shard`. Keep the `.stream()` usage scoped or it can cause borrow issues.
- **`reply.message().await`**: After `ctx.send()`, call `.message().await?` on the returned `SentMessage` to get the actual `Message` object for editing or fetching IDs.
- **EditMessage requires empty attachment vec**: `http.edit_message(ch, msg, &builder, Vec::<CreateAttachment>::new()).await` — the trailing `Vec` is required even if no attachments.
- **Poll expiry in tokio::spawn**: When spawning a background task (e.g. poll expiry), you must clone `db`, `http`, and any IDs before the async move block — they can't be borrowed from the parent scope.
- **Reddit/meme API**: External APIs may return HTML instead of JSON on errors. Always handle decode errors gracefully (return a friendly message, don't panic).
