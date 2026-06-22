use crate::discord::DiscordClient;
use crate::params::*;
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the MCP stdio server (drop-in for mcp-discord)
    Mcp,

    /// Send a message to a Discord text channel
    Send(SendArgs),

    /// Read messages from a Discord text channel
    Read(ReadArgs),

    /// Add a reaction to a message
    React(ReactArgs),

    /// Get information about a Discord server
    #[command(name = "server-info")]
    ServerInfo(ServerInfoArgs),

    /// Send a message via webhook
    #[command(name = "webhook-send")]
    WebhookSend(WebhookSendArgs),

    /// List forum channels in a server
    #[command(name = "forum-channels")]
    ForumChannels(ForumChannelsArgs),

    /// Create a forum post
    #[command(name = "forum-post")]
    ForumPost(ForumPostArgs),
}

#[derive(Args, Debug)]
pub struct SendArgs {
    /// Channel ID to send to
    #[arg(long)]
    pub channel_id: String,

    /// Message content
    #[arg(long)]
    pub message: String,
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    /// Channel ID to read from
    #[arg(long)]
    pub channel_id: String,

    /// Maximum number of messages to retrieve (1-100, default 50)
    #[arg(long, default_value_t = 50)]
    pub limit: u16,
}

#[derive(Args, Debug)]
pub struct ReactArgs {
    /// Channel ID containing the message
    #[arg(long)]
    pub channel_id: String,

    /// Message ID to react to
    #[arg(long)]
    pub message_id: String,

    /// Emoji to add (unicode or name:id for custom)
    #[arg(long)]
    pub emoji: String,
}

#[derive(Args, Debug)]
pub struct ServerInfoArgs {
    /// Guild (server) ID
    #[arg(long)]
    pub guild_id: String,
}

#[derive(Args, Debug)]
pub struct WebhookSendArgs {
    /// Webhook ID
    #[arg(long)]
    pub webhook_id: String,

    /// Webhook token
    #[arg(long)]
    pub webhook_token: String,

    /// Message content
    #[arg(long)]
    pub content: String,

    /// Override username
    #[arg(long)]
    pub username: Option<String>,
}

#[derive(Args, Debug)]
pub struct ForumChannelsArgs {
    /// Guild (server) ID
    #[arg(long)]
    pub guild_id: String,
}

#[derive(Args, Debug)]
pub struct ForumPostArgs {
    /// Forum channel ID
    #[arg(long)]
    pub forum_channel_id: String,

    /// Post title
    #[arg(long)]
    pub title: String,

    /// Post content
    #[arg(long)]
    pub content: String,
}

/// Execute a CLI command, printing pretty JSON to stdout.
/// `Commands::Mcp` is a no-op here — the caller (main) starts the server before
/// reaching this function, but a second caller gets a clean `Ok(())`.
pub async fn run(cmd: Commands, client: &DiscordClient) -> anyhow::Result<()> {
    let v = match cmd {
        Commands::Mcp => return Ok(()),

        Commands::Send(a) => {
            client
                .send(SendParams {
                    channel_id: a.channel_id,
                    message: a.message,
                })
                .await?
        }

        Commands::Read(a) => {
            client
                .read_messages(ReadMessagesParams {
                    channel_id: a.channel_id,
                    limit: a.limit,
                })
                .await?
        }

        Commands::React(a) => {
            client
                .add_reaction(AddReactionParams {
                    channel_id: a.channel_id,
                    message_id: a.message_id,
                    emoji: a.emoji,
                })
                .await?
        }

        Commands::ServerInfo(a) => {
            client
                .get_server_info(GetServerInfoParams { guild_id: a.guild_id })
                .await?
        }

        Commands::WebhookSend(a) => {
            client
                .send_webhook_message(SendWebhookMessageParams {
                    webhook_id: a.webhook_id,
                    webhook_token: a.webhook_token,
                    content: a.content,
                    username: a.username,
                    avatar_url: None,
                    thread_id: None,
                })
                .await?
        }

        Commands::ForumChannels(a) => {
            client
                .get_forum_channels(GetForumChannelsParams { guild_id: a.guild_id })
                .await?
        }

        Commands::ForumPost(a) => {
            client
                .create_forum_post(CreateForumPostParams {
                    forum_channel_id: a.forum_channel_id,
                    title: a.title,
                    content: a.content,
                    tags: None,
                })
                .await?
        }
    };

    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
