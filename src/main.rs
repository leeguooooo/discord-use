// Re-export lib modules into the binary's crate namespace so mod cli and mod mcp
// can reference them as `crate::discord`, `crate::params`, etc.
pub use discord_use::{config, discord, error, params, upgrade};
mod cli;
mod mcp;

use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};

#[derive(Parser, Debug)]
#[command(name = "discord-use", version, about = "Fast REST-only Discord MCP server + CLI")]
struct Cli {
    /// Bot token (overrides env/config)
    #[arg(long, global = true)]
    token: Option<String>,

    /// Compat alias for the old `mcp-discord --config <token>`
    #[arg(long = "config", global = true)]
    config_alias: Option<String>,

    #[command(subcommand)]
    cmd: cli::Commands,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install the ring crypto provider for rustls (required by twilight-http's TLS stack).
    // Must happen before any network call. The ignored Err means "already installed" — harmless;
    // do NOT convert to .expect() or .unwrap().
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Log to stderr — stdout is the MCP JSON-RPC channel and must stay clean.
    // RUST_LOG controls verbosity, e.g. RUST_LOG=discord_use=debug.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let args = Cli::parse();

    // `upgrade` needs no token, so it runs before token resolution.
    if let cli::Commands::Upgrade(ref a) = args.cmd {
        std::process::exit(upgrade::run(a.check, a.json).await);
    }

    // Once-a-day "new version" line, stderr only. Skipped entirely in `mcp`
    // mode: stdout there is the JSON-RPC channel, the host usually doesn't
    // show stderr to the agent anyway, and a stale-cache check (up to 2 s)
    // must not delay the handshake. clap has already exited for --help and
    // --version.
    let is_mcp = matches!(args.cmd, cli::Commands::Mcp(_));
    upgrade::maybe_notify(is_mcp).await;

    let token = config::resolve_token(
        args.token,
        args.config_alias,
        std::env::var("DISCORD_TOKEN").ok(),
        config::read_config_file_token(),
    )?;

    let client = discord::DiscordClient::new(token);

    match args.cmd {
        cli::Commands::Mcp(ref a) => {
            // The label is already visible in `ps` (it is part of our argv); echo it
            // to stderr too so logs from a dozen concurrent servers stay separable.
            match a.label.as_deref() {
                Some(label) => tracing::info!(label, pid = std::process::id(), "discord-use mcp starting"),
                None => tracing::info!(
                    pid = std::process::id(),
                    "discord-use mcp starting (no --label; pass one to tell instances apart in ps)"
                ),
            }
            let service = mcp::DiscordMcp::new(client).serve(stdio()).await?;
            service.waiting().await?;
        }
        other => cli::run(other, &client).await?,
    }

    Ok(())
}
