# discord-use Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `discord-use`, a fast REST-only Rust binary that is a drop-in replacement for the `barryyip0625/mcp-discord` MCP server (no Discord Gateway WebSocket), with full parity across all 22 tools, plus a CLI surface.

**Architecture:** A single Rust crate produces one binary with two surfaces over a shared core: an MCP stdio server (`discord-use mcp`) and a CLI (`discord-use <subcommand>`). All operations are REST calls to `https://discord.com/api/v10` via `twilight-http` (no gateway, no login handshake). The MCP layer (`rmcp`) and CLI layer (`clap`) both call the same async functions in `discord.rs`. The only retained state is a reusable HTTP connection pool.

**Tech Stack:** Rust 2021, `tokio`, `twilight-http` 0.17 + `twilight-model`, `rmcp` 0.11 (+ `schemars` for input schemas), `clap` 4, `serde`/`serde_json`, `thiserror`. Dev: `tokio::test`. Distribution: GitHub Release binaries + `install.sh` (NOT npm).

**Reference spec:** `docs/superpowers/specs/2026-06-22-discord-use-design.md` (read the "Input contract — drop-in fidelity" table; it is the public contract — param names/casing must match byte-for-byte).

---

## Critical drop-in constraint (read before any task)

The thing that makes this a "drop-in" replacement is **the MCP input parameter names and casing**, not the REST mapping. Existing callers (the `discord-hermes` skill, any `mcp__discord__*` reference) pass `channelId`, `messageId`, `avatarURL`, `channelName`, etc. `rmcp` derives each tool's input JSON Schema from a Rust struct via `schemars`, and `schemars` honors serde attributes. Therefore **every input struct gets `#[serde(rename_all = "camelCase")]`**, with explicit per-field renames for the quirks the original server has:

- `discord_send_webhook_message` uses `avatarURL` (not `avatarUrl`) → `#[serde(rename = "avatarURL")]`.
- `discord_create_text_channel` uses `channelName` (camelCase already covers this).
- `discord_create_category` / `discord_edit_category` use `name` (not `categoryName`).

Every tool gets a test asserting its generated schema's property names match the contract. This is the single most important correctness guard in the plan.

---

## File Structure

```
discord-use/
  Cargo.toml                  — crate manifest, pinned deps
  LICENSE-MIT, LICENSE-APACHE — dual license
  README.md                   — install (curl), usage, perf table
  install.sh                  — curl|sh installer (downloads GitHub Release binary)
  .github/workflows/release-binaries.yml — build + attach per-platform binaries
  .gitignore                  — /target
  src/
    main.rs                   — entry; parse args, dispatch mcp-mode vs CLI
    config.rs                 — Token resolution (--token > --config > DISCORD_TOKEN > file) + Bot prefix
    error.rs                  — `Error` enum (thiserror) + → McpError / CLI exit
    params.rs                 — the 22 input structs (serde camelCase + schemars) — shared by mcp+cli
    discord.rs                — `DiscordClient` wrapper over twilight_http::Client; one fn per op
    mcp.rs                    — rmcp server: 22 tools → discord fns
    cli.rs                    — clap subcommands → discord fns
  tests/
    live.rs                   — gated live integration (ignored unless DISCORD_TEST_* set)
```

Each `src/*.rs` has one responsibility. `params.rs` is shared so the casing contract is defined exactly once. Unit tests live inline (`#[cfg(test)] mod tests`) next to the logic they cover; the only separate test file is the live integration suite.

---

## Task 1: Project scaffold

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `.gitignore`, `LICENSE-MIT`, `LICENSE-APACHE`

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "discord-use"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "Fast REST-only Discord MCP server + CLI (no gateway). Drop-in replacement for mcp-discord."
repository = "https://github.com/leeguooooo/discord-use"

[dependencies]
rmcp = { version = "0.11", features = ["server", "transport-io", "macros"] }
schemars = "0.8"
twilight-http = "0.17"
twilight-model = "0.17"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "io-std", "signal"] }
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 2: Create `.gitignore`**

```
/target
```

- [ ] **Step 3: Minimal `src/main.rs` placeholder**

```rust
fn main() {
    println!("discord-use");
}
```

- [ ] **Step 4: Add LICENSE-MIT and LICENSE-APACHE** (standard dual-license texts; copyright "leo / leeguooooo, 2026").

- [ ] **Step 5: Verify it builds**

Run: `cargo build`
Expected: compiles clean (resolves all crate versions). If a pinned minor version is unavailable, run `cargo update` and record the resolved versions in Cargo.lock; do NOT loosen a major version without noting it.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/main.rs LICENSE-MIT LICENSE-APACHE
git commit -m "chore: scaffold discord-use crate with pinned deps"
```

---

## Task 2: Error type (`error.rs`)

**Files:**
- Create: `src/error.rs`

- [ ] **Step 1: Write failing test** (inline in `error.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_token_has_clear_message() {
        let e = Error::MissingToken;
        assert!(e.to_string().contains("token"));
    }
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test error::` → `Error` undefined.

- [ ] **Step 3: Implement**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no Discord bot token provided (set DISCORD_TOKEN, --token, --config, or config file)")]
    MissingToken,
    #[error("Discord API error: {0}")]
    Discord(String),
    #[error("invalid input: {0}")]
    Input(String),
    #[error("config error: {0}")]
    Config(String),
}

impl From<twilight_http::Error> for Error {
    fn from(e: twilight_http::Error) -> Self { Error::Discord(e.to_string()) }
}

pub type Result<T> = std::result::Result<T, Error>;

// Map domain error to rmcp's McpError for the MCP layer.
impl Error {
    pub fn into_mcp(self) -> rmcp::ErrorData {
        rmcp::ErrorData::internal_error(self.to_string(), None)
    }
}
```

- [ ] **Step 4: Run, expect PASS** — `cargo test error::`

- [ ] **Step 5: Commit** — `git commit -am "feat: unified Error type"`

---

## Task 3: Token resolution (`config.rs`)

Resolution order (spec §Auth): `--token` flag → `--config` compat alias → `DISCORD_TOKEN` env → `~/.config/discord-use/config.toml`. Output token is normalized so twilight receives it without a leading `Bot ` (twilight 0.17 adds the prefix itself; double-prefixing breaks auth).

**Files:**
- Create: `src/config.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flag_beats_env() {
        let t = resolve_token(Some("FLAG".into()), None, Some("ENV".into()), None).unwrap();
        assert_eq!(t, "FLAG");
    }
    #[test]
    fn config_alias_beats_env() {
        let t = resolve_token(None, Some("CFG".into()), Some("ENV".into()), None).unwrap();
        assert_eq!(t, "CFG");
    }
    #[test]
    fn env_used_when_no_flags() {
        let t = resolve_token(None, None, Some("ENV".into()), None).unwrap();
        assert_eq!(t, "ENV");
    }
    #[test]
    fn strips_bot_prefix() {
        let t = resolve_token(Some("Bot ABC".into()), None, None, None).unwrap();
        assert_eq!(t, "ABC"); // twilight re-adds "Bot "
    }
    #[test]
    fn missing_is_error() {
        assert!(matches!(resolve_token(None, None, None, None), Err(crate::error::Error::MissingToken)));
    }
}
```

- [ ] **Step 2: Run, expect FAIL.**

- [ ] **Step 3: Implement**

```rust
use crate::error::{Error, Result};

/// Resolve the bot token by precedence and strip any leading "Bot " (twilight re-adds it).
/// `file_token` is the value already read from the config file (None if absent/unreadable),
/// passed in so this fn stays pure and unit-testable.
pub fn resolve_token(
    flag: Option<String>,
    config_alias: Option<String>,
    env: Option<String>,
    file_token: Option<String>,
) -> Result<String> {
    let raw = flag.or(config_alias).or(env).or(file_token).ok_or(Error::MissingToken)?;
    let trimmed = raw.trim();
    let bare = trimmed.strip_prefix("Bot ").unwrap_or(trimmed);
    if bare.is_empty() { return Err(Error::MissingToken); }
    Ok(bare.to_string())
}

/// Read token from ~/.config/discord-use/config.toml (key: token = "..."). Returns None on any miss.
pub fn read_config_file_token() -> Option<String> {
    let path = dirs_config_path()?;
    let body = std::fs::read_to_string(path).ok()?;
    // tiny parse: find a line `token = "..."`
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("token") {
            if let Some(eq) = rest.trim_start().strip_prefix('=') {
                return Some(eq.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn dirs_config_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(std::path::Path::new(&home).join(".config/discord-use/config.toml"))
}
```

- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** — `git commit -am "feat: token resolution + Bot-prefix normalization"`

---

## Task 4: Input param structs + casing contract (`params.rs`)

This task encodes the drop-in contract. Define one struct per tool that takes params (tools with only optional/no params still get a struct for uniformity). Apply `#[serde(rename_all = "camelCase")]` and explicit renames for quirks. Then write schema-casing tests for the load-bearing ones.

**Files:**
- Create: `src/params.rs`

- [ ] **Step 1: Write failing casing tests** (these guard drop-in compat)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;

    fn prop_names(schema: schemars::schema::RootSchema) -> Vec<String> {
        schema.schema.object.map(|o| o.properties.keys().cloned().collect()).unwrap_or_default()
    }

    #[test]
    fn send_uses_channelId_and_message() {
        let names = prop_names(schema_for!(SendParams));
        assert!(names.contains(&"channelId".to_string()));
        assert!(names.contains(&"message".to_string()));
        assert!(!names.contains(&"channel_id".to_string()));
    }

    #[test]
    fn webhook_message_uses_avatarURL_quirk() {
        let names = prop_names(schema_for!(SendWebhookMessageParams));
        assert!(names.contains(&"avatarURL".to_string()), "must reproduce avatarURL casing, got {names:?}");
        assert!(names.contains(&"webhookToken".to_string()));
    }

    #[test]
    fn create_text_channel_uses_channelName() {
        let names = prop_names(schema_for!(CreateTextChannelParams));
        assert!(names.contains(&"channelName".to_string()));
        assert!(names.contains(&"guildId".to_string()));
    }
}
```

- [ ] **Step 2: Run, expect FAIL** (structs undefined).

- [ ] **Step 3: Implement all 22 param structs.** Each derives `Deserialize, schemars::JsonSchema` and uses camelCase. Match the contract table in the spec exactly. Full set:

```rust
use serde::Deserialize;
use schemars::JsonSchema;

macro_rules! params {
    ($name:ident { $($body:tt)* }) => {
        #[derive(Debug, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($body)* }
    };
}

params!(LoginParams { pub token: Option<String> });
params!(GetServerInfoParams { pub guild_id: String });
params!(SendParams { pub channel_id: String, pub message: String });
params!(ReadMessagesParams {
    pub channel_id: String,
    #[serde(default = "default_limit")] pub limit: u16,   // default 50, clamp 1..=100 in discord.rs
});
fn default_limit() -> u16 { 50 }
params!(DeleteMessageParams { pub channel_id: String, pub message_id: String, pub reason: Option<String> });
params!(AddReactionParams { pub channel_id: String, pub message_id: String, pub emoji: String });
params!(AddMultipleReactionsParams { pub channel_id: String, pub message_id: String, pub emojis: Vec<String> });
params!(RemoveReactionParams { pub channel_id: String, pub message_id: String, pub emoji: String, pub user_id: Option<String> });
params!(CreateTextChannelParams { pub guild_id: String, pub channel_name: String, pub topic: Option<String> });
params!(DeleteChannelParams { pub channel_id: String, pub reason: Option<String> });
params!(CreateCategoryParams { pub guild_id: String, pub name: String, pub position: Option<u16>, pub reason: Option<String> });
params!(EditCategoryParams { pub category_id: String, pub name: Option<String>, pub position: Option<u16>, pub reason: Option<String> });
params!(DeleteCategoryParams { pub category_id: String, pub reason: Option<String> });
params!(CreateWebhookParams { pub channel_id: String, pub name: String, pub avatar: Option<String>, pub reason: Option<String> });
params!(EditWebhookParams { pub webhook_id: String, pub name: Option<String>, pub channel_id: Option<String>, pub avatar: Option<String>, pub webhook_token: Option<String>, pub reason: Option<String> });
params!(DeleteWebhookParams { pub webhook_id: String, pub webhook_token: Option<String>, pub reason: Option<String> });

// QUIRK: original uses `avatarURL`, not camelCase `avatarUrl`. Explicit rename.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendWebhookMessageParams {
    pub webhook_id: String,
    pub webhook_token: String,
    pub content: String,
    pub username: Option<String>,
    #[serde(rename = "avatarURL")] pub avatar_url: Option<String>,
    pub thread_id: Option<String>,
}

params!(GetForumChannelsParams { pub guild_id: String });
params!(CreateForumPostParams { pub forum_channel_id: String, pub title: String, pub content: String, pub tags: Option<Vec<String>> });
params!(GetForumPostParams { pub thread_id: String });
params!(ReplyToForumParams { pub thread_id: String, pub message: String });
params!(DeleteForumPostParams { pub thread_id: String, pub reason: Option<String> });
```

> Note: `#[serde(rename_all = "camelCase")]` turns `channel_id` → `channelId`, `message_id` → `messageId`, `guild_id` → `guildId`, `channel_name` → `channelName`, `webhook_token` → `webhookToken`, `forum_channel_id` → `forumChannelId`, `thread_id` → `threadId`, `user_id` → `userId`, `category_id` → `categoryId`. Verify each against the spec table.

- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** — `git commit -am "feat: input param structs with drop-in camelCase contract + schema tests"`

---

## Task 5: Discord REST wrapper (`discord.rs`)

A `DiscordClient` holding a `twilight_http::Client`, with one async method per operation returning `serde_json::Value` (so both MCP and CLI serialize uniformly). Implement in category sub-tasks; commit after each. Each method maps a param struct → twilight call → JSON result.

**Files:**
- Create: `src/discord.rs`

- [ ] **Step 0: Skeleton + constructor**

```rust
use crate::error::{Error, Result};
use crate::params::*;
use twilight_http::Client;
use twilight_model::id::Id;
use serde_json::{json, Value};

pub struct DiscordClient { http: Client }

impl DiscordClient {
    pub fn new(token: String) -> Self { Self { http: Client::new(token) } }

    pub async fn login(&self, _p: LoginParams) -> Result<Value> {
        let me = self.http.current_user().await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        Ok(json!({ "id": me.id.to_string(), "username": me.name, "loggedIn": true }))
    }
}

// Parse a string into a twilight Id<T>. Returns Input error on bad snowflake.
fn parse_id<T>(s: &str) -> Result<Id<T>> {
    s.parse::<u64>().ok().and_then(Id::new_checked).ok_or_else(|| Error::Input(format!("invalid id: {s}")))
}
```

### Sub-task 5a: Messaging + emoji helper

- [ ] Test (TDD): emoji parser + limit clamp are pure logic — test them.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clamps_limit() {
        assert_eq!(clamp_limit(0), 1);
        assert_eq!(clamp_limit(50), 50);
        assert_eq!(clamp_limit(999), 100);
    }
    #[test]
    fn parses_unicode_and_custom_emoji() {
        assert!(matches!(parse_emoji("👍"), ReactionEmoji::Unicode(_)));
        // custom emoji form "name:id"
        match parse_emoji("blob:12345") {
            ReactionEmoji::Custom { id, .. } => assert_eq!(id, "12345"),
            _ => panic!("expected custom"),
        }
    }
}
```

- [ ] Implement helpers + methods:

```rust
pub(crate) fn clamp_limit(n: u16) -> u16 { n.clamp(1, 100) }

pub(crate) enum ReactionEmoji { Unicode(String), Custom { name: String, id: String } }

pub(crate) fn parse_emoji(s: &str) -> ReactionEmoji {
    // custom form "name:id" (id all digits); otherwise unicode
    if let Some((name, id)) = s.rsplit_once(':') {
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
            return ReactionEmoji::Custom { name: name.to_string(), id: id.to_string() };
        }
    }
    ReactionEmoji::Unicode(s.to_string())
}

impl DiscordClient {
    pub async fn send(&self, p: SendParams) -> Result<Value> {
        let ch = parse_id(&p.channel_id)?;
        let msg = self.http.create_message(ch).content(&p.message)
            .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        Ok(json!({ "id": msg.id.to_string(), "channelId": p.channel_id, "content": msg.content }))
    }

    pub async fn read_messages(&self, p: ReadMessagesParams) -> Result<Value> {
        let ch = parse_id(&p.channel_id)?;
        let limit = clamp_limit(p.limit);
        let msgs = self.http.channel_messages(ch).limit(limit)
            .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        let out: Vec<Value> = msgs.iter().map(|m| json!({
            "id": m.id.to_string(),
            "author": m.author.name,
            "content": m.content,
            "timestamp": m.timestamp.iso_8601().to_string(),
        })).collect();
        Ok(json!({ "channelId": p.channel_id, "messages": out }))
    }

    pub async fn delete_message(&self, p: DeleteMessageParams) -> Result<Value> {
        let ch = parse_id(&p.channel_id)?; let m = parse_id(&p.message_id)?;
        let req = self.http.delete_message(ch, m);
        // attach audit-log reason if provided (twilight: .reason(&str))
        match p.reason.as_deref() { Some(r) => req.reason(r).await?, None => req.await? };
        Ok(json!({ "deleted": true, "messageId": p.message_id }))
    }
}
```

> twilight method-name caveat: confirm `.limit(u16)` vs `.limit(u16)?` (some builders return `Result`) and `.reason()` availability against the resolved 0.17.x docs as you implement; adjust `?`/await chaining to match. The JSON shapes above are ours, not twilight's — keep them stable since callers may parse them.

- [ ] Run tests (`cargo test discord::`), expect PASS. Commit: `git commit -am "feat: discord messaging ops + emoji/limit helpers"`

### Sub-task 5b: Reactions (with partial-failure aggregation)

- [ ] Test the aggregation logic with an injected per-emoji closure (pure, no network):

```rust
#[test]
fn multi_reaction_continues_on_failure_and_reports_each() {
    // add_multiple aggregates results; emoji "bad" fails, others succeed
    let results = aggregate_reactions(vec!["a".into(),"bad".into(),"c".into()],
        |e| if e == "bad" { Err("nope".into()) } else { Ok(()) });
    assert_eq!(results.iter().filter(|r| r.ok).count(), 2);
    assert_eq!(results.iter().filter(|r| !r.ok).count(), 1);
}
```

- [ ] Implement `aggregate_reactions` (sync helper over a closure) + the three methods (`add_reaction`, `add_multiple_reactions` looping through `aggregate_reactions` with the real twilight call, `remove_reaction`). Use `twilight_model::http::interaction`/`RequestReactionType` for the emoji; map `ReactionEmoji::Unicode` → `RequestReactionType::Unicode { name }` and `Custom` → `RequestReactionType::Custom { id, name }`. `add_multiple_reactions` returns `json!({ "results": [{ "emoji": .., "ok": bool, "error": Option<String> }] })`.
- [ ] Run tests, expect PASS. Commit: `git commit -am "feat: reaction ops with per-emoji partial-failure reporting"`

### Sub-task 5c: Channels

- [ ] Implement `create_text_channel` (`create_guild_channel(guild, &name)`, set kind text + optional topic) and `delete_channel` (`delete_channel(id)`, optional `.reason()`). Return JSON with new/affected channel id + name. No new pure logic → covered by live tests. Commit: `git commit -am "feat: text channel create/delete"`

### Sub-task 5d: Categories

- [ ] Implement `create_category` (`create_guild_channel` with kind `GuildCategory`, optional position), `edit_category` (`update_channel(id).name()/.position()`), `delete_category` (`delete_channel(id)`). Commit: `git commit -am "feat: category create/edit/delete"`

### Sub-task 5e: Webhooks

- [ ] Implement `create_webhook` (`create_webhook(channel, &name)`), `edit_webhook` (`update_webhook(id).name()/.channel_id()`), `delete_webhook` (`delete_webhook(id)`), `send_webhook_message` (`execute_webhook(webhook_id, &token).content(&content)` + optional `.username()/.avatar_url()`; if `thread_id` set, `.thread_id(parse_id)`). Commit: `git commit -am "feat: webhook crud + execute"`

### Sub-task 5f: Forum

- [ ] Implement:
  - `get_forum_channels` → `guild_channels(guild)`, filter `channel.kind == ChannelType::GuildForum`, return id+name list.
  - `create_forum_post` → `create_forum_thread(forum_channel, &title)` with message content (+ optional applied tags parsed to `Id`); return thread id.
  - `get_forum_post` → `channel(thread)` + `channel_messages(thread)`.
  - `reply_to_forum` → `create_message(thread).content(&message)`.
  - `delete_forum_post` → `delete_channel(thread)`.
- [ ] Commit: `git commit -am "feat: forum channel + post ops"`

> Throughout Task 5: verify each twilight 0.17.x builder method name against docs.rs as you go (e.g. `update_channel` vs `update_guild_channel`, `create_forum_thread` exact name/signature). The plan pins the operation and JSON shape; the exact twilight call is confirmed at implementation time. Keep all returned JSON shapes stable.

---

## Task 6: MCP server (`mcp.rs`)

Register all 22 tools with `rmcp`, each delegating to the matching `DiscordClient` method. Tool names MUST be the originals (`discord_send`, etc.) — set via `#[tool(name = "...")]` since Rust fn names can't start with `discord_` ambiguously (use explicit names to be safe).

**Files:**
- Create: `src/mcp.rs`

- [ ] **Step 1: Implement the server** (pattern — full file lists all 22; one shown):

```rust
use crate::discord::DiscordClient;
use crate::params::*;
use rmcp::{handler::server::tool::ToolRouter, handler::server::wrapper::Parameters,
           model::*, tool, tool_handler, tool_router, ServerHandler, ErrorData as McpError};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiscordMcp { client: Arc<DiscordClient>, tool_router: ToolRouter<Self> }

#[tool_router]
impl DiscordMcp {
    pub fn new(client: DiscordClient) -> Self {
        Self { client: Arc::new(client), tool_router: Self::tool_router() }
    }

    #[tool(name = "discord_send", description = "Sends a message to a specified Discord text channel")]
    async fn send(&self, Parameters(p): Parameters<SendParams>) -> Result<CallToolResult, McpError> {
        let v = self.client.send(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v).map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    // ... 21 more tools, identical shape: #[tool(name = "discord_<x>", description = "<original desc>")]
    //     async fn <x>(&self, Parameters(p): Parameters<XParams>) -> Result<CallToolResult, McpError>
    //     { let v = self.client.<x>(p).await.map_err(|e| e.into_mcp())?; Ok(CallToolResult::success(vec![Content::json(v)?])) }
}

#[tool_handler]
impl ServerHandler for DiscordMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("REST-only Discord integration (no gateway).".into()),
        }
    }
}
```

Use the exact `description` strings captured from the live server (in the spec's source-of-truth; copy verbatim so tool listings match).

- [ ] **Step 2: Compile check** — `cargo build`. (Tool dispatch is integration-tested live; rmcp wiring has no unit logic to TDD.)
- [ ] **Step 3: Commit** — `git commit -am "feat: rmcp server exposing all 22 tools (drop-in names)"`

---

## Task 7: CLI (`cli.rs`)

clap subcommands mirroring the tools, calling the same `DiscordClient` methods, printing the JSON result (pretty to stdout). Names can be ergonomic (`send`, `read`, `react`, `webhook send`, …) — the CLI is for humans/scripts, not drop-in.

**Files:**
- Create: `src/cli.rs`

- [ ] **Step 1: Implement** a clap `#[derive(Parser)]` with a `Commands` enum covering the common ops (at minimum: `send`, `read`, `react`, `server-info`, `webhook-send`; the rest are nice-to-have and can be added without breaking anything). Each arm builds the matching `*Params` and calls the client, then `println!("{}", serde_json::to_string_pretty(&v)?)`.
- [ ] **Step 2: Compile check** — `cargo build`.
- [ ] **Step 3: Commit** — `git commit -am "feat: CLI surface for direct/scripted use"`

---

## Task 8: Entry point (`main.rs`)

Dispatch: `discord-use mcp` → start the stdio MCP server; anything else → CLI. Token resolved once via `config.rs`.

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement**

```rust
mod config; mod error; mod params; mod discord; mod mcp; mod cli;

use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};

#[derive(Parser)]
#[command(name = "discord-use", version)]
struct Cli {
    /// Bot token (overrides env/config)
    #[arg(long, global = true)] token: Option<String>,
    /// Compat alias for the old `mcp-discord --config <token>`
    #[arg(long, global = true)] config: Option<String>,
    #[command(subcommand)] cmd: cli::Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_writer(std::io::stderr).with_ansi(false).init();
    let args = Cli::parse();
    let token = config::resolve_token(
        args.token.clone(), args.config.clone(),
        std::env::var("DISCORD_TOKEN").ok(), config::read_config_file_token(),
    )?;
    let client = discord::DiscordClient::new(token);

    match args.cmd {
        cli::Commands::Mcp => {
            let service = mcp::DiscordMcp::new(client).serve(stdio()).await?;
            service.waiting().await?;
        }
        other => cli::run(other, &client).await?,
    }
    Ok(())
}
```

> `cli::Commands` must include an `Mcp` variant. `anyhow` needs adding to deps (or map errors manually). `config::resolve_token` returns `crate::error::Error` → add `impl From<Error> for anyhow::Error` or use `?` with anyhow's blanket impl (Error implements std::error::Error via thiserror, so `?` works under anyhow).

- [ ] **Step 2: Build + smoke** — `cargo build`; `cargo run -- --help` shows subcommands incl. `mcp`.
- [ ] **Step 3: Commit** — `git commit -am "feat: entrypoint dispatch (mcp vs cli) + token wiring"`

---

## Task 9: OSS packaging (README, license already done, CI, installer)

**Files:**
- Create: `README.md`, `install.sh`, `.github/workflows/release-binaries.yml`

- [ ] **Step 1: `README.md`** — leads with the curl install line, then a usage section (MCP config snippet for Claude + Codex), then the perf table from the spec (mark RSS/startup numbers as "measured" placeholders to be filled from Task 10 benchmark). Document that it's a drop-in replacement for `mcp-discord`.
- [ ] **Step 2: `install.sh`** — detect OS/arch, download the matching `.tar.gz` from the latest GitHub Release, verify `.sha256`, install to `~/.local/bin/discord-use`. Model on the user's `leeguooooo/Mailbox` install.sh.
- [ ] **Step 3: `.github/workflows/release-binaries.yml`** — on tag push, build `x86_64`/`aarch64` for macOS + Linux, package `.tar.gz` + `.sha256`, attach to the GitHub Release. Model on `leeguooooo/Mailbox` `.github/workflows/release-binaries.yml`.
- [ ] **Step 4: Commit** — `git commit -am "docs+ci: README, installer, release-binaries workflow"`

---

## Task 10: Live verification + cutover (run OUTSIDE the sandbox)

These steps hit the real Discord API and edit live config; the human runs them.

- [ ] **Step 1: Build release** — `cargo build --release`; binary at `target/release/discord-use`.
- [ ] **Step 2: Smoke MCP** — `DISCORD_TOKEN=<token> echo '{"jsonrpc":"2.0",...}' | target/release/discord-use mcp` (or wire it into Claude as a temp server and call `discord_login`). Confirm it returns the bot identity with no gateway connection.
- [ ] **Step 3: Live non-destructive exercise** — against a throwaway test guild/channel: `discord_send`, `discord_read_messages`, `discord_add_reaction`, `discord_get_server_info`, `discord_send_webhook_message`, `discord_create_forum_post`. (Destructive tools — delete_* — exercised only if the user provides a disposable target.)
- [ ] **Step 4: Benchmark** — record idle RSS (`ps`/Activity Monitor) and cold-start latency vs the old npx server; fill the README perf table.
- [ ] **Step 5: Cutover Claude** — edit `~/.claude.json` `discord` entry:
  ```json
  "discord": { "type": "stdio", "command": "/Users/leo/.local/bin/discord-use", "args": ["mcp"], "env": { "DISCORD_TOKEN": "<token>" } }
  ```
- [ ] **Step 6: Cutover Codex** — edit `~/.codex/config.toml` `[mcp_servers.discord]` to `command = "discord-use"`, `args = ["mcp"]`, and add the token under `env`.
- [ ] **Step 7: Verify** in a fresh Claude + Codex session that `mcp__discord__discord_send` etc. work, then kill any lingering old `npx mcp-discord` / node processes. Confirm only the lightweight `discord-use` process is resident.
- [ ] **Step 8: Publish** — push to `github.com/leeguooooo/discord-use` (public), tag `v0.1.0` to trigger the release workflow.

---

## Notes for the implementer

- Confirm each `twilight-http` 0.17.x builder method name/signature against docs.rs at implementation time; the plan pins the operation + our JSON output shape, which are the stable contract.
- Keep `params.rs` casing exactly per the spec's contract table — the schema-casing tests in Task 4 are the guard; never "tidy up" `avatarURL`/`channelName`.
- Use `@superpowers:test-driven-development` discipline: every pure-logic helper (token resolution, limit clamp, emoji parse, reaction aggregation, schema casing) gets a failing test first. Network-bound twilight calls are covered by the gated live suite, not mocked.
- Commit after every green step.
