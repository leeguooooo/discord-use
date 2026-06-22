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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_token_has_clear_message() {
        let e = Error::MissingToken;
        assert!(e.to_string().contains("token"));
    }
}
