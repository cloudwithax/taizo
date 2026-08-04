# AGENTS.md

## Project

Rust Discord bot (serenity 0.12 + poise 0.6) with PostgreSQL (sqlx). Single crate, no workspace.

## Rules

- All visible copy (embed titles, descriptions, field names, field values) must be strictly lowercase. No exceptions.
- Please reference the rules for [when you make changes](#when-you-make-changes) everytime you make changes
- Dont over think shit.
- Use menu buttons and embeds for most interactions
- If a feature you need exists in a crate, use it. Otherwise make it yourself.

## Build and run

```bash
cargo build
cargo run
```

## When you make changes
- Use `cargo clippy && cargo check && cargo build` to verify changes
- Note any pitfalls you experienced in the library/modules in AGENTS.md
- Commit changes
- Push changes to main
- After pushing, restart the service: `sudo systemctl restart taizo.service` (or use `/restart` in Discord — owner only)

## Service

Runs as a systemd service (`taizo.service`). Owner-only Discord commands `/restart` and `/stop` control it.

```bash
sudo systemctl status taizo.service    # check status
sudo systemctl restart taizo.service   # restart
sudo systemctl stop taizo.service      # stop
sudo journalctl -u taizo.service -f    # tail logs
```


## Schema

`schema.sql` is embedded via `include_str!` and auto-applied on every startup (split by `;`). No migrations tool — edits to `schema.sql` take effect next boot.

## Environment

Requires `.env` with `TOKEN`, `DATABASE_URL`, and `OWNER_ID` (loaded by dotenvy).

## Commands

Register in `main.rs` under `framework.options().commands`. Each module lives in `src/commands/`:
- `economy.rs` — bank accounts, balance, work/crime/slut, gambling, leaderboard
- `fun.rs` — say, choose, hug, kiss, embed, poll, snipe, reddit, owoify, etc.
- `info.rs` — about, uptime, invite, privacy, vote, support
- `moderation.rs` — ban, kick, mute, warn, purge, setwelcome/setleave
- `omnimod.rs` — AI-powered text moderation (omnimod) + image NSFW classification + OCR text extraction
- `owner.rs` — restart, stop (owner only)
- `utility.rs` — help (paginated buttons), ping, serverinfo, userinfo, avatar, whois

## Image Classifier

- Model: `models/model.onnx` (89MB, converted from bonker-js tfjs via h5 → ONNX)
- Runtime: `tract-onnx` (pure Rust, no system deps)
- Preprocessing: bilinear resize with align_corners to 299×299, /255 normalization
- Classification: hentai + porn > 0.5 threshold (matches bonker-js behavior)
- GIF support: decodes up to 10 frames, flags if any frame exceeds threshold
- Integrated into `omnimod::handle_message` — classifies image attachments when omnimod is enabled

## OCR (Optical Character Recognition)

- Runtime: `rusty-tesseract` (wrapper around Tesseract OCR binary)
- System dependency: requires `tesseract` binary installed (v5.5.0+)
- Extracts text from images and GIFs (up to 10 frames)
- Extracted text runs through pre-stage moderation
- If flagged, escalates to LLM (stage1/stage2) for nuanced review
- Results logged with `ocr_pre_stage` and `ocr_stage2` stage labels
- Actions: `ocr_banned_and_deleted`, `ocr_message_deleted`, `ocr_crisis_dm_sent`, `ocr_logged_for_review`
- Runs independently of NSFW classification — both checks happen on every image

## OCR Pitfalls

- **Tesseract binary required**: `rusty-tesseract` shells out to the `tesseract` binary. Must be installed system-wide (`apt install tesseract-ocr` or equivalent).
- **PSM and OEM modes**: Use PSM 6 (uniform block of text) and OEM 3 (default engine) for best results on Discord screenshots.
- **GIF frame limit**: Only processes first 10 frames to avoid performance issues on long GIFs.
- **Empty text handling**: If OCR returns empty string, skips moderation pipeline (no false positives on images without text).
- **Performance**: OCR is synchronous and can be slow on large images. Consider running in `tokio::task::spawn_blocking` if latency becomes an issue.

## Tract-ONNX Pitfalls

- `RunnableModel` is a type alias for `SimplePlan<F, O>` requiring 2 generic params
- `into_runnable()` returns `Arc<RunnableModel<F, O>>`, not the raw type
- `Runnable` trait is implemented for `Arc<TypedRunnableModel>`, not for `&T`
- For ONNX models: use `Arc<TypedRunnableModel>` (aka `Arc<RunnableModel<TypedFact, Box<dyn TypedOp>>`)
- `TypedRunnableModel` is in `tract_core::model::typed` but accessible via `tract_onnx::prelude::*`
- GIF frames from `image` crate are RGBA; convert to RGB before classification

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

## Omnimod / Self-Hosted LLM Pitfalls

- **API**: Self-hosted at `https://omnimodapi.clxud.dev/v1/chat/completions` using llama.cpp backend. API key env var is `OMNIMOD_API_KEY`.
- **Model**: `Qwen3.5-4B-Q4_K_M.gguf` for both stage 1 (triage) and stage 2 (adjudication). Model ID is the full path: `/home/clxud/models/Qwen3.5-4B-Q4_K_M.gguf`.
- **`enable_thinking` parameter**: Not supported directly in the request body. Use `chat_template_kwargs: {"enable_thinking": false}` instead to disable thinking mode.
- **Cloudflare blocks Python urllib**: Default Python User-Agent gets 403'd. Set `User-Agent: curl/8.14.1` and `Accept: */*` headers in test scripts.
- **Prompt tuning for small models**: The 4B model needs explicit direction rules. Imperatives and information-sharing must be explicitly called out as other-directed (BAN), not self-directed (CRISIS). The model defaults to CRISIS for ambiguous direction without explicit guidance.
- **Benchmark**: `test_omnimod.py` tests stage 2 prompt only. Run with `OMNIMOD_API_KEY=<key> python3 test_omnimod.py`. Target: 27/27.
