# discord-use — Design Spec

**Date:** 2026-06-22
**Status:** Approved (approach A, full parity)
**Replaces:** `barryyip0625/mcp-discord` (npm `mcp-discord`, TypeScript + discord.js)

## Problem

The Discord MCP currently wired into both Claude Code (`~/.claude.json`) and Codex
(`~/.codex/config.toml`) is `npx -y mcp-discord` (barryyip0625/mcp-discord). It uses
discord.js, which **logs in a full Gateway client (a persistent WebSocket session)**
purely to perform operations that are, in fact, ordinary REST calls.

Consequences:
- Every agent that boots the MCP spawns its own Node process holding a live Discord
  Gateway session. N agents → N resident Node processes (~100–150 MB RSS each).
- Slow cold start: each process waits for the gateway `READY` event before serving.
- Each process consumes one of the bot's limited concurrent gateway sessions.

The workload is **entirely request/response** (send / read recent / manage). There is no
need to passively receive events, so the Gateway connection is pure overhead.

## Goals

- Drop-in replacement: same MCP server name (`discord`) and **identical tool names**, so
  existing consumers (e.g. the `discord-hermes` skill, any `mcp__discord__*` reference)
  work unchanged.
- Eliminate the resident Gateway: REST-only, no persistent WebSocket, no login handshake.
- Full parity with all 22 tools of the original.
- Ship as a clean open-source Rust project, distributed via GitHub Release binaries.

## Non-goals

- Real-time event listening / bot command framework (would require the Gateway). Out of
  scope by design — if ever needed, it is a separate opt-in daemon, not this binary.
- Voice, sharding, presence.

## Approach (selected: A — stateless REST-only)

A single Rust binary, `discord-use`, with two surfaces over one shared core:

- **MCP mode** (`discord-use mcp`) — stdio MCP server (the agents launch this).
- **CLI mode** (`discord-use <subcommand> …`) — direct/scriptable use; matches the
  user's existing `chrome-use` / `wechat-use` / `iphone-use` tool family.

No Gateway WebSocket. Each operation is one (or a few) REST calls to
`https://discord.com/api/v10`. The only retained state is a reusable HTTP connection pool.

Rejected alternatives:
- **B. Shared daemon + per-agent shim** — only worth it to amortize a *shared* gateway
  connection; REST is stateless so there's nothing to share. Adds socket lifecycle
  complexity, and the codex sandbox blocks `socket.bind('/tmp/*.sock')`.
- **C. CLI-only via Bash** — leanest (process per call) but loses MCP tool schemas; kept
  as the secondary mode of A rather than the primary.

## Stack

- **`twilight-http`** + **`twilight-model`** — the REST-only, no-gateway Discord client.
  Ships a correct 429 rate-limiter, so backoff is handled for free.
- **`rmcp`** — official Rust MCP SDK; stdio transport, tool registration.
- **`clap`** — arg parsing / CLI subcommands.
- **`tokio`** — async runtime.
- **`serde` / `serde_json`** — (de)serialization.
- **`thiserror`** — error type.
- Dev: standard `#[tokio::test]` / `#[test]`. Pure logic is unit-tested directly; twilight
  HTTP calls are exercised by the gated live integration suite rather than mocked (see Testing).

## Module layout (single crate)

```
src/
  main.rs      — clap entry; dispatch mcp-mode vs CLI subcommands
  config.rs    — token resolution (--token > DISCORD_TOKEN > config file) + "Bot " prefixing
  discord.rs   — thin wrapper over twilight_http::Client; one async fn per operation
                 (the shared core called by BOTH mcp.rs and cli.rs — no duplicated logic)
  mcp.rs       — rmcp server: tool registration, JSON params → discord fn, result → MCP content
  cli.rs       — clap subcommands → same discord fns; prints JSON or human output
  error.rs     — thiserror unified error → MCP tool-error / CLI exit code
```

**Data flow (MCP):** client → stdio JSON-RPC → `rmcp` dispatch → tool fn → `discord::Client`
→ `twilight-http` → discord.com → serialize → stdout. Stateless.

## Tool surface — full parity (22 tools)

Tool names are kept identical to barryyip0625/mcp-discord for drop-in compatibility.

| Tool | REST mapping |
|---|---|
| `discord_login` | GET `/users/@me` — validate token, return bot identity (no gateway) |
| `discord_get_server_info` | GET `/guilds/{id}` + GET `/guilds/{id}/channels` |
| `discord_send` | POST `/channels/{ch}/messages` |
| `discord_read_messages` | GET `/channels/{ch}/messages?limit=N` |
| `discord_delete_message` | DELETE `/channels/{ch}/messages/{msg}` |
| `discord_add_reaction` | PUT `/channels/{ch}/messages/{msg}/reactions/{emoji}/@me` |
| `discord_add_multiple_reactions` | loop of `add_reaction` |
| `discord_remove_reaction` | DELETE `/channels/{ch}/messages/{msg}/reactions/{emoji}/@me` |
| `discord_create_text_channel` | POST `/guilds/{id}/channels` `{type:0,name,parent_id?,topic?}` |
| `discord_delete_channel` | DELETE `/channels/{id}` |
| `discord_create_category` | POST `/guilds/{id}/channels` `{type:4,name}` |
| `discord_edit_category` | PATCH `/channels/{id}` `{name,position?}` |
| `discord_delete_category` | DELETE `/channels/{id}` |
| `discord_create_webhook` | POST `/channels/{ch}/webhooks` `{name}` |
| `discord_edit_webhook` | PATCH `/webhooks/{id}` `{name?,channel_id?}` |
| `discord_delete_webhook` | DELETE `/webhooks/{id}` |
| `discord_send_webhook_message` | POST `/webhooks/{id}/{token}` `{content,username?,avatar_url?}` |
| `discord_get_forum_channels` | GET `/guilds/{id}/channels` filtered to type 15 (GUILD_FORUM) |
| `discord_create_forum_post` | POST `/channels/{forum}/threads` `{name,message:{content},applied_tags?}` |
| `discord_get_forum_post` | GET `/channels/{thread}` + GET messages |
| `discord_reply_to_forum` | POST `/channels/{thread}/messages` `{content}` |
| `discord_delete_forum_post` | DELETE `/channels/{thread}` |

All endpoints are REST and require no Gateway connection.

### Input contract — drop-in fidelity (authoritative, captured from the live server)

"Drop-in" depends on the **MCP input parameter names/casing matching exactly** — callers
like the `discord-hermes` skill pass `channelId` + `message`, not `channel_id` + `content`.
The MCP tool input schemas below were captured verbatim from the running
barryyip0625/mcp-discord server and **must be reproduced byte-for-byte** (param names,
casing, required vs optional, defaults). The REST mapping above is internal; this table is
the public contract.

| Tool | Required params | Optional params |
|---|---|---|
| `discord_login` | — | `token` |
| `discord_get_server_info` | `guildId` | — |
| `discord_send` | `channelId`, `message` | — |
| `discord_read_messages` | `channelId` | `limit` (number, default 50, min 1, max 100) |
| `discord_delete_message` | `channelId`, `messageId` | `reason` |
| `discord_add_reaction` | `channelId`, `messageId`, `emoji` | — |
| `discord_add_multiple_reactions` | `channelId`, `messageId`, `emojis` (string[]) | — |
| `discord_remove_reaction` | `channelId`, `messageId`, `emoji` | `userId` |
| `discord_create_text_channel` | `guildId`, `channelName` | `topic` |
| `discord_delete_channel` | `channelId` | `reason` |
| `discord_create_category` | `guildId`, `name` | `position` (number), `reason` |
| `discord_edit_category` | `categoryId` | `name`, `position` (number), `reason` |
| `discord_delete_category` | `categoryId` | `reason` |
| `discord_create_webhook` | `channelId`, `name` | `avatar`, `reason` |
| `discord_edit_webhook` | `webhookId` | `name`, `channelId`, `avatar`, `webhookToken`, `reason` |
| `discord_delete_webhook` | `webhookId` | `webhookToken`, `reason` |
| `discord_send_webhook_message` | `webhookId`, `webhookToken`, `content` | `username`, `avatarURL`, `threadId` |
| `discord_get_forum_channels` | `guildId` | — |
| `discord_create_forum_post` | `forumChannelId`, `title`, `content` | `tags` (string[]) |
| `discord_get_forum_post` | `threadId` | — |
| `discord_reply_to_forum` | `threadId`, `message` | — |
| `discord_delete_forum_post` | `threadId` | `reason` |

Note the inconsistent casing in the original (`avatarURL` on webhook-message vs `avatar`
on create/edit-webhook; `channelName` vs `name`) — reproduce these quirks exactly rather
than normalizing them, or drop-in compatibility breaks.

**`discord_add_multiple_reactions` partial-failure semantics:** add reactions sequentially;
on a per-emoji failure (403/429/invalid emoji), continue the remaining emojis and return a
per-emoji success/failure report rather than aborting on the first error.

## Auth / token handling

Resolution order: `--token` flag → `DISCORD_TOKEN` env → `~/.config/discord-use/config.toml`.

- The default MCP config passes the token via the `env` block, **not argv** — the current
  setup leaks the bot token into process listings and two plaintext config files.
- The old server is launched as `npx -y mcp-discord --config <BOT_TOKEN>` (verified in both
  `~/.claude.json` and `~/.codex/config.toml`). `discord-use` therefore accepts a
  `--config <token>` compat alias (identical flag, same positional meaning) so an existing
  config keeps working if only the command/package name is swapped.
- Bot tokens are sent as `Authorization: Bot <token>`; `config.rs` normalizes the prefix.

## Error handling

twilight surfaces typed REST errors (429 rate-limit, 403 missing perms, 404 not found).
- MCP mode: return an MCP tool-error carrying Discord's error code + message.
- CLI mode: exit nonzero, write the message to stderr.
- 429 backoff is handled by twilight's built-in rate-limiter.

## Testing

- **Unit (sandbox-safe):** token resolution precedence, `Bot ` prefixing, input-schema
  casing (drop-in contract, via `schemars::schema_for!`), emoji parsing, limit clamping,
  multi-reaction partial-failure aggregation, result serialization. No network, no mocking.
- **Live integration (run outside sandbox):** gated behind `DISCORD_TEST_*` env vars;
  sends/reads/manages against a real test guild/channel — this is what covers the actual
  twilight HTTP calls.

## Performance targets (documented in README)

| Metric | old (Node + discord.js) | discord-use (Rust REST) |
|---|---|---|
| Idle RSS | ~100–150 MB | < 5 MB |
| Cold start → first response | seconds (waits for gateway READY) | < 100 ms |
| Discord gateway sessions used | 1 per agent | 0 |

A before/after benchmark table ships in the README.

## Distribution (per global rule — GitHub Releases, not npm)

- License: dual MIT / Apache-2.0 (Rust convention).
- CI builds per-platform binaries (macOS arm64/x64, Linux x64/arm64) → attached to the
  GitHub Release as `.tar.gz` + `.sha256`.
- `install.sh` for `curl -fsSL https://raw.githubusercontent.com/<owner>/discord-use/main/install.sh | sh`.
- README leads with the curl install line and the perf table. README stays Markdown
  (GitHub-dictated format).

## Cutover plan

1. Build + live-verify `discord-use` (full parity) against a real channel.
2. Point `~/.claude.json` and `~/.codex/config.toml` `discord` entries at
   `discord-use mcp`, token in the `env` block.
3. Remove the old `npx -y mcp-discord` entry → its resident Node processes die on reload.
