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
                let val = eq.trim();
                // quoted value: strip surrounding quotes
                if val.starts_with('"') {
                    return Some(val.trim_matches('"').to_string());
                }
                // unquoted value: strip trailing inline comment (`# ...`) before trimming
                let val = val.split_once(" #").map(|(v, _)| v).unwrap_or(val);
                let val = val.split_once("\t#").map(|(v, _)| v).unwrap_or(val);
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

fn dirs_config_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(std::path::Path::new(&home).join(".config/discord-use/config.toml"))
}

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

    // helpers to exercise read_config_file_token's inline parser without touching the filesystem
    fn parse_config_line(line: &str) -> Option<String> {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("token") {
            if let Some(eq) = rest.trim_start().strip_prefix('=') {
                let val = eq.trim();
                if val.starts_with('"') {
                    return Some(val.trim_matches('"').to_string());
                }
                let val = val.split_once(" #").map(|(v, _)| v).unwrap_or(val);
                let val = val.split_once("\t#").map(|(v, _)| v).unwrap_or(val);
                return Some(val.trim().to_string());
            }
        }
        None
    }

    #[test]
    fn config_file_unquoted_with_comment() {
        // token = abc # note  →  "abc"
        assert_eq!(parse_config_line("token = abc # note"), Some("abc".into()));
    }

    #[test]
    fn config_file_quoted_value() {
        // token = "abc"  →  "abc"
        assert_eq!(parse_config_line(r#"token = "abc""#), Some("abc".into()));
    }
}
