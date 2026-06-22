// Re-export lib modules into the binary's crate namespace so mod cli and mod mcp
// can reference them as `crate::discord`, `crate::params`, etc.
pub use discord_use::{config, discord, error, params};
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

    let token = config::resolve_token(
        args.token,
        args.config_alias,
        std::env::var("DISCORD_TOKEN").ok(),
        config::read_config_file_token(),
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
