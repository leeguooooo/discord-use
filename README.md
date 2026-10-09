# discord-use

Fast REST-only Discord MCP server + CLI. Drop-in replacement for `mcp-discord` — no gateway, no Node.js, ~4 MB idle RSS.

```sh
curl -fsSL https://raw.githubusercontent.com/leeguooooo/discord-use/main/install.sh | sh
```

---

## Why it exists

The standard Discord MCP (`barryyip0625/mcp-discord`) opens a full Gateway WebSocket on startup — a persistent connection that:

- Costs **100–150 MB RSS per agent** (one Node process per agent that boots the server)
- **Delays startup** by several seconds waiting for the `READY` event
- Consumes one of the bot's limited concurrent gateway sessions

All 22 tools are ordinary REST calls. `discord-use` skips the gateway entirely: one binary, zero resident connections, sub-second start.

## Performance

| Metric | `mcp-discord` (Node + discord.js) | `discord-use` (Rust REST) |
|---|---|---|
| Binary / package size | ~90 MB on-disk (node_modules) | **5.8 MB** (single binary, measured here; strip+lto release profile) |
| Idle RSS per agent | ~100–150 MB (documented/typical for discord.js gateway client) | **4.1 MB** (measured: 4224 KB RSS, `ps -o rss=`) |
| Cold start → first response | seconds (waits for gateway `READY`) | **immediate** — no gateway handshake; serves on stdin |
| Discord gateway sessions | 1 per agent | **0** |

_Binary size and RSS measured on macOS aarch64 from this build. Node/discord.js figures are documented typical values for a discord.js gateway client, not freshly benchmarked here._

---

## Installation

### Prebuilt binary (recommended)

```sh
curl -fsSL https://raw.githubusercontent.com/leeguooooo/discord-use/main/install.sh | sh
```

Installs to `~/.local/bin/discord-use`. Prebuilt binaries: macOS (Apple Silicon and Intel) and Linux (x86_64 / aarch64).

Override install directory:

```sh
DISCORD_USE_INSTALL_DIR=/usr/local/bin \
  curl -fsSL https://raw.githubusercontent.com/leeguooooo/discord-use/main/install.sh | sh
```

### Build from source

Requires [Rust](https://rustup.rs/) (stable).

```sh
git clone https://github.com/leeguooooo/discord-use
cd discord-use
cargo build --release
# binary at: target/release/discord-use
```

Maintainers release with `scripts/release.sh <version>` (`--dry-run` first): it tags, waits for the release binaries, then syncs the plugin marketplace.

### Upgrading

```sh
discord-use upgrade          # install the latest release where this binary lives, refresh the skill
discord-use upgrade --check  # just report: discord-use 0.1.0 -> 0.2.0 / is up to date
discord-use upgrade --json   # same, as JSON (includes where the skill is installed)
```

Once a day a CLI command may print one `discord-use X is available` line on **stderr** (cached in
`~/.cache/discord-use/update-check.json`, 2 s timeout). `discord-use mcp` never checks. Turn it off
with `DISCORD_USE_NO_UPDATE_CHECK=1` or the family-wide `USE_NO_UPDATE_CHECK=1` (also off when `CI`
is set).

---

## MCP configuration

### Claude Code (`~/.claude.json`)

```json
{
  "mcpServers": {
    "discord": {
      "type": "stdio",
      "command": "discord-use",
      "args": ["mcp"],
      "env": {
        "DISCORD_TOKEN": "<your-bot-token>"
      }
    }
  }
}
```

### Codex (`~/.codex/config.toml`)

```toml
[mcp_servers.discord]
command = "discord-use"
args    = ["mcp"]

[mcp_servers.discord.env]
DISCORD_TOKEN = "<your-bot-token>"
```

The token is passed via the environment, not argv — keeping it out of process listings. The old `--config <token>` flag (used by `npx mcp-discord`) still works as a compat alias.

### Telling instances apart: `--label`

Under stdio transport every MCP client session spawns its own server process, so a
machine running a dozen agent sessions ends up with a dozen identical
`discord-use mcp` rows in `ps` — same command, no way to tell which belongs to
which. Memory is not the problem (~1.4 MB each); identification is.

Add a `--label` to say who a server belongs to. It is free-form and not
interpreted — it exists so the string lands in `ps` output:

```json
"args": ["mcp", "--label", "work-account"]
```

```console
$ ps -Ao pid,command | grep 'discord-use mcp'
96236 discord-use mcp --label work-account
96237 discord-use mcp --label team-support-bot
```

The label is also logged to stderr at startup, so logs from concurrent servers
stay separable. Never put the token in it — argv is world-readable.

---

## Auth

Resolution order (first wins):

1. `--token <TOKEN>` flag
2. `--config <TOKEN>` flag (compat alias for drop-in replacement of `npx mcp-discord --config <token>`)
3. `DISCORD_TOKEN` environment variable
4. `~/.config/discord-use/config.toml` — key `token = "..."`

---

## CLI usage

```sh
# Send a message
discord-use send --channel-id 1234567890 --message "Hello from discord-use"

# Read recent messages
discord-use read --channel-id 1234567890 --limit 20

# Add a reaction
discord-use react --channel-id 1234567890 --message-id 9876543210 --emoji 👍

# Server info
discord-use server-info --guild-id 1111111111

# Send via webhook
discord-use webhook-send \
  --webhook-id 2222222222 \
  --webhook-token abc123 \
  --content "Webhook message"

# Start the MCP server (used by Claude / Codex)
discord-use mcp
```

Run `discord-use --help` or `discord-use <subcommand> --help` for full options.

---

## Tools (22)

| Tool | Description |
|---|---|
| `discord_login` | Validate token and return bot identity |
| `discord_get_server_info` | Get guild info + channel list |
| `discord_send` | Send a message to a text channel |
| `discord_read_messages` | Read recent messages from a channel |
| `discord_delete_message` | Delete a message |
| `discord_add_reaction` | Add an emoji reaction |
| `discord_add_multiple_reactions` | Add several reactions; continues on per-emoji failure |
| `discord_remove_reaction` | Remove an emoji reaction |
| `discord_create_text_channel` | Create a text channel in a guild |
| `discord_delete_channel` | Delete a channel |
| `discord_create_category` | Create a category in a guild |
| `discord_edit_category` | Rename / reposition a category |
| `discord_delete_category` | Delete a category |
| `discord_create_webhook` | Create a webhook for a channel |
| `discord_edit_webhook` | Update a webhook's name or channel |
| `discord_delete_webhook` | Delete a webhook |
| `discord_send_webhook_message` | Execute a webhook (send a message as the webhook) |
| `discord_get_forum_channels` | List forum channels in a guild |
| `discord_create_forum_post` | Create a new forum thread |
| `discord_get_forum_post` | Get a forum thread and its messages |
| `discord_reply_to_forum` | Post a reply in a forum thread |
| `discord_delete_forum_post` | Delete a forum thread |

Tool names, parameter names, and casing are **identical** to `barryyip0625/mcp-discord` — existing callers (e.g. skills referencing `mcp__discord__*`) work unchanged.

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

<!-- use-family -->
## The `*-use` family

Small, composable CLIs that give an AI agent hands on one real thing. Same shape
everywhere: `curl … install.sh | sh` to install, `npx skills add leeguooooo/<name>`
to teach your agent, JSON on stdout.

| Repo | Gives your agent |
|---|---|
| [chrome-use](https://github.com/leeguooooo/chrome-use) | A real browser — logged-in sessions, forms, scraping, screenshots |
| [mail-use](https://github.com/leeguooooo/mail-use) | Email — read, search, send, triage across Gmail / QQ / 163 / any IMAP |
| [iphone-use](https://github.com/leeguooooo/iphone-use) | A real iPhone — tap, type, screenshot, pull on-device data |
| [wechat-use](https://github.com/leeguooooo/wechat-use) | WeChat on macOS — send messages, query contacts and history |
| [cookie-use](https://github.com/leeguooooo/cookie-use) | Many logged-in accounts per site — capture, switch, apply sessions |
| [profile-use](https://github.com/leeguooooo/profile-use) | Your personal profile, safely — fill signup / KYC / checkout forms |
| [bitwarden-use](https://github.com/leeguooooo/bitwarden-use) | Bitwarden / Vaultwarden — headless passkey (FIDO2) login |
| [chatgpt-use](https://github.com/leeguooooo/chatgpt-use) | Your ChatGPT subscription as a coding-agent backend — no API key |
| [computer-use](https://github.com/leeguooooo/computer-use) | The macOS desktop itself |
| [pixcake-use](https://github.com/leeguooooo/pixcake-use) | Read-only PixCake probing — snapshot / diff / SQLite inspection |
