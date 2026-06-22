mod cli;
mod config;
mod discord;
mod error;
mod mcp;
mod params;

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
    // This must happen before any network calls; if already installed the error is harmless.
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Log to stderr so stdout stays clean for MCP JSON-RPC
    tracing_subscriber::fmt()
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
