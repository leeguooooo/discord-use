---
name: discord-use
description: Fast REST-only Discord CLI + MCP server (single ~6 MB Rust binary, no gateway, no Node.js). Use when the user wants to send or read Discord messages, add reactions, inspect a server/guild, manage channels, categories, webhooks, or forum posts from the command line or an agent. Drop-in replacement for mcp-discord — same 22 tool names. Needs a bot token via DISCORD_TOKEN, --token, or ~/.config/discord-use/config.toml.
allowed-tools: Bash(discord-use:*)
---

# discord-use

REST-only Discord CLI + MCP server in one binary. No gateway WebSocket: sub-second start, ~4 MB
idle RSS, zero gateway sessions consumed.

**Install / self-heal:** if `discord-use` is missing, install from the GitHub Release (no npm, no
token), then retry:

```sh
curl -fsSL https://raw.githubusercontent.com/leeguooooo/discord-use/main/install.sh | sh
```

## Auth

First match wins: `--token <TOKEN>` → `--config <TOKEN>` (mcp-discord compat) → `DISCORD_TOKEN`
env → `~/.config/discord-use/config.toml` (`token = "..."`). Prefer the env var or config file so
the token stays out of process listings.

## CLI usage

```sh
discord-use send --channel-id <id> --message "text"
discord-use read --channel-id <id> --limit 20
discord-use react --channel-id <id> --message-id <id> --emoji 👍
discord-use server-info --guild-id <id>
discord-use webhook-send --webhook-id <id> --webhook-token <token> --content "text"
```

`discord-use --help` / `discord-use <subcommand> --help` for the full surface (channels,
categories, webhooks, forum posts — 22 operations total).

## MCP server mode

For hosts that speak MCP instead of a shell:

```sh
discord-use mcp     # stdio MCP server; token via DISCORD_TOKEN env
```

Tool names/parameters are identical to `barryyip0625/mcp-discord`, so existing `mcp__discord__*`
callers work unchanged.

## Agent etiquette

- Sending a message is outward-facing and irreversible — confirm content and target channel with
  the user before sending unless they already gave you the exact message and destination.
- Never print the bot token.

## Upgrade

When any `discord-use` command prints `discord-use X is available`, tell the user and offer to run
`discord-use upgrade` (it updates the CLI and this skill; no token needed). Check without changing
anything: `discord-use upgrade --check`. The user may also just say "升级 discord-use" /
"upgrade discord-use". (`discord-use mcp` never checks, so the notice only shows on CLI calls.)

If the skill came from somewhere `upgrade` can't refresh:
- Claude Code plugin: `claude plugin update discord-use@leeguooooo-plugins`
- Whole family: `curl -fsSL https://raw.githubusercontent.com/leeguooooo/plugins/main/upgrade-use-family.sh | sh`
