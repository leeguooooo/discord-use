use crate::discord::DiscordClient;
use crate::params::*;
use rmcp::{
    ErrorData as McpError,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    tool, tool_handler, tool_router, ServerHandler,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiscordMcp {
    client: Arc<DiscordClient>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl DiscordMcp {
    pub fn new(client: DiscordClient) -> Self {
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(name = "discord_login", description = "Logs in to Discord using the configured token")]
    async fn discord_login(
        &self,
        Parameters(p): Parameters<LoginParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.login(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_get_server_info",
        description = "Retrieves detailed information about a Discord server including channels and member count"
    )]
    async fn discord_get_server_info(
        &self,
        Parameters(p): Parameters<GetServerInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.get_server_info(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_send",
        description = "Sends a message to a specified Discord text channel"
    )]
    async fn discord_send(
        &self,
        Parameters(p): Parameters<SendParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.send(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_read_messages",
        description = "Retrieves messages from a Discord text channel with a configurable limit"
    )]
    async fn discord_read_messages(
        &self,
        Parameters(p): Parameters<ReadMessagesParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.read_messages(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_delete_message",
        description = "Deletes a specific message from a Discord text channel"
    )]
    async fn discord_delete_message(
        &self,
        Parameters(p): Parameters<DeleteMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.delete_message(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_add_reaction",
        description = "Adds an emoji reaction to a specific Discord message"
    )]
    async fn discord_add_reaction(
        &self,
        Parameters(p): Parameters<AddReactionParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.add_reaction(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_add_multiple_reactions",
        description = "Adds multiple emoji reactions to a Discord message at once"
    )]
    async fn discord_add_multiple_reactions(
        &self,
        Parameters(p): Parameters<AddMultipleReactionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.add_multiple_reactions(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_remove_reaction",
        description = "Removes a specific emoji reaction from a Discord message"
    )]
    async fn discord_remove_reaction(
        &self,
        Parameters(p): Parameters<RemoveReactionParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.remove_reaction(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_create_text_channel",
        description = "Creates a new text channel in a Discord server with an optional topic"
    )]
    async fn discord_create_text_channel(
        &self,
        Parameters(p): Parameters<CreateTextChannelParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.create_text_channel(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_delete_channel",
        description = "Deletes a Discord channel with an optional reason"
    )]
    async fn discord_delete_channel(
        &self,
        Parameters(p): Parameters<DeleteChannelParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.delete_channel(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_create_category",
        description = "Creates a new category in a Discord server."
    )]
    async fn discord_create_category(
        &self,
        Parameters(p): Parameters<CreateCategoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.create_category(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_edit_category",
        description = "Edits an existing Discord category (name and position)."
    )]
    async fn discord_edit_category(
        &self,
        Parameters(p): Parameters<EditCategoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.edit_category(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_delete_category",
        description = "Deletes a Discord category by ID."
    )]
    async fn discord_delete_category(
        &self,
        Parameters(p): Parameters<DeleteCategoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.delete_category(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_create_webhook",
        description = "Creates a new webhook for a Discord channel"
    )]
    async fn discord_create_webhook(
        &self,
        Parameters(p): Parameters<CreateWebhookParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.create_webhook(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_edit_webhook",
        description = "Edits an existing webhook for a Discord channel"
    )]
    async fn discord_edit_webhook(
        &self,
        Parameters(p): Parameters<EditWebhookParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.edit_webhook(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_delete_webhook",
        description = "Deletes an existing webhook for a Discord channel"
    )]
    async fn discord_delete_webhook(
        &self,
        Parameters(p): Parameters<DeleteWebhookParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.delete_webhook(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_send_webhook_message",
        description = "Sends a message to a Discord channel using a webhook"
    )]
    async fn discord_send_webhook_message(
        &self,
        Parameters(p): Parameters<SendWebhookMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.send_webhook_message(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_get_forum_channels",
        description = "Lists all forum channels in a specified Discord server (guild)"
    )]
    async fn discord_get_forum_channels(
        &self,
        Parameters(p): Parameters<GetForumChannelsParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.get_forum_channels(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_create_forum_post",
        description = "Creates a new post in a Discord forum channel with optional tags"
    )]
    async fn discord_create_forum_post(
        &self,
        Parameters(p): Parameters<CreateForumPostParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.create_forum_post(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_get_forum_post",
        description = "Retrieves details about a forum post including its messages"
    )]
    async fn discord_get_forum_post(
        &self,
        Parameters(p): Parameters<GetForumPostParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.get_forum_post(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_reply_to_forum",
        description = "Adds a reply to an existing forum post or thread"
    )]
    async fn discord_reply_to_forum(
        &self,
        Parameters(p): Parameters<ReplyToForumParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.reply_to_forum(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }

    #[tool(
        name = "discord_delete_forum_post",
        description = "Deletes a forum post or thread with an optional reason"
    )]
    async fn discord_delete_forum_post(
        &self,
        Parameters(p): Parameters<DeleteForumPostParams>,
    ) -> Result<CallToolResult, McpError> {
        let v = self.client.delete_forum_post(p).await.map_err(|e| e.into_mcp())?;
        Ok(CallToolResult::success(vec![Content::json(v)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?]))
    }
}

#[tool_handler]
impl ServerHandler for DiscordMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "REST-only Discord integration (no gateway). \
                 Provides 22 tools for messaging, reactions, channels, webhooks, and forums."
                    .into(),
            ),
        }
    }
}
