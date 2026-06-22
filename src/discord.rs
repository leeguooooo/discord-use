use crate::error::{Error, Result};
use crate::params::*;
use serde_json::{Value, json};
use twilight_http::Client;
use twilight_http::request::AuditLogReason;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::ChannelType;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, EmojiMarker, GuildMarker, MessageMarker, UserMarker, WebhookMarker};

pub struct DiscordClient {
    http: Client,
}

impl DiscordClient {
    pub fn new(token: String) -> Self {
        Self { http: Client::new(token) }
    }

    // --- auth / identity ---

    pub async fn login(&self, _p: LoginParams) -> Result<Value> {
        let me = self.http.current_user().await?.model().await
            .map_err(|e| Error::Discord(e.to_string()))?;
        Ok(json!({ "id": me.id.to_string(), "username": me.name, "loggedIn": true }))
    }

    // --- server info ---

    pub async fn get_server_info(&self, p: GetServerInfoParams) -> Result<Value> {
        let gid: Id<GuildMarker> = parse_id(&p.guild_id)?;
        let guild = self.http.guild(gid).await?.model().await
            .map_err(|e| Error::Discord(e.to_string()))?;
        let channels = self.http.guild_channels(gid).await?.model().await
            .map_err(|e| Error::Discord(e.to_string()))?;
        let ch_list: Vec<Value> = channels.iter().map(|c| json!({
            "id": c.id.to_string(),
            "name": c.name.as_deref().unwrap_or(""),
            "type": u8::from(c.kind),
        })).collect();
        Ok(json!({
            "id": guild.id.to_string(),
            "name": guild.name,
            "channels": ch_list,
        }))
    }

    // --- messaging ---

    pub async fn send(&self, p: SendParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        let msg = self.http.create_message(ch).content(&p.message)
            .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        Ok(json!({
            "id": msg.id.to_string(),
            "channelId": p.channel_id,
            "content": msg.content,
        }))
    }

    pub async fn read_messages(&self, p: ReadMessagesParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        let limit = clamp_limit(p.limit);
        let msgs = self.http.channel_messages(ch).limit(limit)
            .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        let out: Vec<Value> = msgs.iter().map(|m| json!({
            "id": m.id.to_string(),
            "author": m.author.name,
            "content": m.content,
            "timestamp": m.timestamp.iso_8601().to_string(),
        })).collect();
        Ok(json!({ "channelId": p.channel_id, "messages": out }))
    }

    pub async fn delete_message(&self, p: DeleteMessageParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        let m: Id<MessageMarker> = parse_id(&p.message_id)?;
        match p.reason.as_deref() {
            Some(r) => self.http.delete_message(ch, m).reason(r).await?,
            None    => self.http.delete_message(ch, m).await?,
        };
        Ok(json!({ "deleted": true, "messageId": p.message_id }))
    }

    // --- reactions ---

    pub async fn add_reaction(&self, p: AddReactionParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        let m: Id<MessageMarker> = parse_id(&p.message_id)?;
        react_add(&self.http, ch, m, &p.emoji).await?;
        Ok(json!({ "added": true, "emoji": p.emoji }))
    }

    pub async fn add_multiple_reactions(&self, p: AddMultipleReactionsParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        let m: Id<MessageMarker> = parse_id(&p.message_id)?;
        let mut results: Vec<Value> = Vec::new();
        for emoji_s in &p.emojis {
            match react_add(&self.http, ch, m, emoji_s).await {
                Ok(_)  => results.push(json!({ "emoji": emoji_s, "ok": true, "error": null })),
                Err(e) => results.push(json!({ "emoji": emoji_s, "ok": false, "error": e.to_string() })),
            }
        }
        Ok(json!({ "results": results }))
    }

    pub async fn remove_reaction(&self, p: RemoveReactionParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        let m: Id<MessageMarker> = parse_id(&p.message_id)?;
        match p.user_id.as_deref() {
            Some(uid) => {
                let user: Id<UserMarker> = parse_id(uid)?;
                react_remove_user(&self.http, ch, m, &p.emoji, user).await?;
            }
            None => {
                react_remove_me(&self.http, ch, m, &p.emoji).await?;
            }
        }
        Ok(json!({ "removed": true, "emoji": p.emoji }))
    }

    // --- channels ---

    pub async fn create_text_channel(&self, p: CreateTextChannelParams) -> Result<Value> {
        let gid: Id<GuildMarker> = parse_id(&p.guild_id)?;
        let ch = if let Some(ref topic) = p.topic {
            self.http.create_guild_channel(gid, &p.channel_name)
                .kind(ChannelType::GuildText)
                .topic(topic)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?
        } else {
            self.http.create_guild_channel(gid, &p.channel_name)
                .kind(ChannelType::GuildText)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?
        };
        Ok(json!({
            "id": ch.id.to_string(),
            "name": ch.name.as_deref().unwrap_or(""),
            "type": "text",
        }))
    }

    pub async fn delete_channel(&self, p: DeleteChannelParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        match p.reason.as_deref() {
            Some(r) => { self.http.delete_channel(ch).reason(r).await?; }
            None    => { self.http.delete_channel(ch).await?; }
        }
        Ok(json!({ "deleted": true, "channelId": p.channel_id }))
    }

    // --- categories ---

    pub async fn create_category(&self, p: CreateCategoryParams) -> Result<Value> {
        let gid: Id<GuildMarker> = parse_id(&p.guild_id)?;
        let ch = match (p.position, p.reason.as_deref()) {
            (Some(pos), Some(r)) => self.http.create_guild_channel(gid, &p.name)
                .kind(ChannelType::GuildCategory).position(pos as u64)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (Some(pos), None) => self.http.create_guild_channel(gid, &p.name)
                .kind(ChannelType::GuildCategory).position(pos as u64)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, Some(r)) => self.http.create_guild_channel(gid, &p.name)
                .kind(ChannelType::GuildCategory)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, None) => self.http.create_guild_channel(gid, &p.name)
                .kind(ChannelType::GuildCategory)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
        };
        Ok(json!({
            "id": ch.id.to_string(),
            "name": ch.name.as_deref().unwrap_or(""),
            "type": "category",
        }))
    }

    pub async fn edit_category(&self, p: EditCategoryParams) -> Result<Value> {
        let cat: Id<ChannelMarker> = parse_id(&p.category_id)?;
        let ch = match (p.name.as_deref(), p.position, p.reason.as_deref()) {
            (Some(name), Some(pos), Some(r)) => self.http.update_channel(cat)
                .name(name).position(pos as u64)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (Some(name), Some(pos), None) => self.http.update_channel(cat)
                .name(name).position(pos as u64)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (Some(name), None, Some(r)) => self.http.update_channel(cat)
                .name(name)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (Some(name), None, None) => self.http.update_channel(cat)
                .name(name)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, Some(pos), Some(r)) => self.http.update_channel(cat)
                .position(pos as u64)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, Some(pos), None) => self.http.update_channel(cat)
                .position(pos as u64)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, None, Some(r)) => self.http.update_channel(cat)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, None, None) => self.http.update_channel(cat)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
        };
        Ok(json!({
            "id": ch.id.to_string(),
            "name": ch.name.as_deref().unwrap_or(""),
            "type": "category",
        }))
    }

    pub async fn delete_category(&self, p: DeleteCategoryParams) -> Result<Value> {
        let cat: Id<ChannelMarker> = parse_id(&p.category_id)?;
        match p.reason.as_deref() {
            Some(r) => { self.http.delete_channel(cat).reason(r).await?; }
            None    => { self.http.delete_channel(cat).await?; }
        }
        Ok(json!({ "deleted": true, "categoryId": p.category_id }))
    }

    // --- webhooks ---

    pub async fn create_webhook(&self, p: CreateWebhookParams) -> Result<Value> {
        let ch: Id<ChannelMarker> = parse_id(&p.channel_id)?;
        let wh = match p.reason.as_deref() {
            Some(r) => self.http.create_webhook(ch, &p.name)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            None => self.http.create_webhook(ch, &p.name)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
        };
        Ok(json!({
            "id": wh.id.to_string(),
            "name": wh.name.as_deref().unwrap_or(""),
            "channelId": wh.channel_id.to_string(),
            "token": wh.token.as_deref().unwrap_or(""),
        }))
    }

    pub async fn edit_webhook(&self, p: EditWebhookParams) -> Result<Value> {
        let wid: Id<WebhookMarker> = parse_id(&p.webhook_id)?;
        let ch_id: Option<Id<ChannelMarker>> = p.channel_id.as_deref()
            .map(parse_id).transpose()?;
        let wh = match (p.name.as_deref(), ch_id, p.reason.as_deref()) {
            (Some(name), Some(cid), Some(r)) => self.http.update_webhook(wid)
                .name(name).channel_id(cid)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (Some(name), Some(cid), None) => self.http.update_webhook(wid)
                .name(name).channel_id(cid)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (Some(name), None, Some(r)) => self.http.update_webhook(wid)
                .name(name)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (Some(name), None, None) => self.http.update_webhook(wid)
                .name(name)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, Some(cid), Some(r)) => self.http.update_webhook(wid)
                .channel_id(cid)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, Some(cid), None) => self.http.update_webhook(wid)
                .channel_id(cid)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, None, Some(r)) => self.http.update_webhook(wid)
                .reason(r).await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
            (None, None, None) => self.http.update_webhook(wid)
                .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?,
        };
        Ok(json!({
            "id": wh.id.to_string(),
            "name": wh.name.as_deref().unwrap_or(""),
            "channelId": wh.channel_id.to_string(),
        }))
    }

    pub async fn delete_webhook(&self, p: DeleteWebhookParams) -> Result<Value> {
        let wid: Id<WebhookMarker> = parse_id(&p.webhook_id)?;
        match p.reason.as_deref() {
            Some(r) => { self.http.delete_webhook(wid).reason(r).await?; }
            None    => { self.http.delete_webhook(wid).await?; }
        }
        Ok(json!({ "deleted": true, "webhookId": p.webhook_id }))
    }

    pub async fn send_webhook_message(&self, p: SendWebhookMessageParams) -> Result<Value> {
        let wid: Id<WebhookMarker> = parse_id(&p.webhook_id)?;
        let thread_id: Option<Id<ChannelMarker>> = p.thread_id.as_deref()
            .map(parse_id).transpose()?;
        // Build all optional fields as locals so lifetimes extend over the await.
        let username = p.username.as_deref();
        let avatar   = p.avatar_url.as_deref();
        let mut req  = self.http.execute_webhook(wid, &p.webhook_token).content(&p.content);
        if let Some(u) = username { req = req.username(u); }
        if let Some(a) = avatar   { req = req.avatar_url(a); }
        if let Some(tid) = thread_id { req = req.thread_id(tid); }
        req.await?;
        Ok(json!({ "sent": true, "webhookId": p.webhook_id }))
    }

    // --- forum ---

    pub async fn get_forum_channels(&self, p: GetForumChannelsParams) -> Result<Value> {
        let gid: Id<GuildMarker> = parse_id(&p.guild_id)?;
        let channels = self.http.guild_channels(gid).await?.model().await
            .map_err(|e| Error::Discord(e.to_string()))?;
        let forums: Vec<Value> = channels.iter()
            .filter(|c| c.kind == ChannelType::GuildForum)
            .map(|c| json!({
                "id": c.id.to_string(),
                "name": c.name.as_deref().unwrap_or(""),
            }))
            .collect();
        Ok(json!({ "guildId": p.guild_id, "forums": forums }))
    }

    pub async fn create_forum_post(&self, p: CreateForumPostParams) -> Result<Value> {
        let forum: Id<ChannelMarker> = parse_id(&p.forum_channel_id)?;
        let thread = self.http
            .create_forum_thread(forum, &p.title)
            .message()
            .content(&p.content)
            .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        Ok(json!({
            "threadId": thread.channel.id.to_string(),
            "title": thread.channel.name.as_deref().unwrap_or(""),
            "messageId": thread.message.id.to_string(),
        }))
    }

    pub async fn get_forum_post(&self, p: GetForumPostParams) -> Result<Value> {
        let thread: Id<ChannelMarker> = parse_id(&p.thread_id)?;
        let ch = self.http.channel(thread).await?.model().await
            .map_err(|e| Error::Discord(e.to_string()))?;
        let msgs = self.http.channel_messages(thread).limit(50)
            .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        let out: Vec<Value> = msgs.iter().map(|m| json!({
            "id": m.id.to_string(),
            "author": m.author.name,
            "content": m.content,
            "timestamp": m.timestamp.iso_8601().to_string(),
        })).collect();
        Ok(json!({
            "threadId": p.thread_id,
            "name": ch.name.as_deref().unwrap_or(""),
            "messages": out,
        }))
    }

    pub async fn reply_to_forum(&self, p: ReplyToForumParams) -> Result<Value> {
        let thread: Id<ChannelMarker> = parse_id(&p.thread_id)?;
        let msg = self.http.create_message(thread).content(&p.message)
            .await?.model().await.map_err(|e| Error::Discord(e.to_string()))?;
        Ok(json!({
            "id": msg.id.to_string(),
            "threadId": p.thread_id,
            "content": msg.content,
        }))
    }

    pub async fn delete_forum_post(&self, p: DeleteForumPostParams) -> Result<Value> {
        let thread: Id<ChannelMarker> = parse_id(&p.thread_id)?;
        match p.reason.as_deref() {
            Some(r) => { self.http.delete_channel(thread).reason(r).await?; }
            None    => { self.http.delete_channel(thread).await?; }
        }
        Ok(json!({ "deleted": true, "threadId": p.thread_id }))
    }
}

// --- pure helpers (TDD-tested) ---

/// Clamp a message limit to the Discord-allowed range 1..=100.
pub(crate) fn clamp_limit(n: u16) -> u16 {
    n.clamp(1, 100)
}

/// Internal representation of a parsed emoji string.
pub(crate) enum ReactionEmoji {
    Unicode(String),
    Custom { name: String, id: String },
}

/// Parse "name:12345" into Custom, anything else into Unicode.
pub(crate) fn parse_emoji(s: &str) -> ReactionEmoji {
    if let Some((name, id)) = s.rsplit_once(':') {
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
            return ReactionEmoji::Custom { name: name.to_string(), id: id.to_string() };
        }
    }
    ReactionEmoji::Unicode(s.to_string())
}

/// Result entry for add_multiple_reactions partial-failure aggregation.
pub(crate) struct ReactionResult {
    pub ok: bool,
    pub error: Option<String>,
}

/// Pure aggregation helper: run `f` over each emoji, continuing on failure.
/// `f` is a sync closure returning Result<(), String>.
pub(crate) fn aggregate_reactions<F>(emojis: Vec<String>, f: F) -> Vec<ReactionResult>
where
    F: Fn(&str) -> std::result::Result<(), String>,
{
    emojis.iter().map(|e| {
        match f(e) {
            Ok(_)    => ReactionResult { ok: true, error: None },
            Err(msg) => ReactionResult { ok: false, error: Some(msg) },
        }
    }).collect()
}

// --- private helpers ---

/// Parse a string snowflake into a twilight Id<T>. Returns Input error on invalid/zero value.
fn parse_id<T>(s: &str) -> Result<Id<T>> {
    s.parse::<u64>()
        .ok()
        .and_then(Id::new_checked)
        .ok_or_else(|| Error::Input(format!("invalid snowflake id: {s}")))
}

/// Add a reaction using the correct `RequestReactionType` variant.
/// Reaction builder borrows `RequestReactionType` which borrows strings, so
/// we build both in one async fn to keep all lifetimes in scope.
async fn react_add(
    http: &Client,
    ch: Id<ChannelMarker>,
    m: Id<MessageMarker>,
    emoji_s: &str,
) -> Result<()> {
    match parse_emoji(emoji_s) {
        ReactionEmoji::Unicode(ref name) => {
            let rrt = RequestReactionType::Unicode { name };
            http.create_reaction(ch, m, &rrt).await?;
        }
        ReactionEmoji::Custom { ref name, ref id } => {
            let eid: Id<EmojiMarker> = parse_id(id)?;
            let rrt = RequestReactionType::Custom { id: eid, name: Some(name) };
            http.create_reaction(ch, m, &rrt).await?;
        }
    }
    Ok(())
}

/// Remove the bot's own reaction.
async fn react_remove_me(
    http: &Client,
    ch: Id<ChannelMarker>,
    m: Id<MessageMarker>,
    emoji_s: &str,
) -> Result<()> {
    match parse_emoji(emoji_s) {
        ReactionEmoji::Unicode(ref name) => {
            let rrt = RequestReactionType::Unicode { name };
            http.delete_current_user_reaction(ch, m, &rrt).await?;
        }
        ReactionEmoji::Custom { ref name, ref id } => {
            let eid: Id<EmojiMarker> = parse_id(id)?;
            let rrt = RequestReactionType::Custom { id: eid, name: Some(name) };
            http.delete_current_user_reaction(ch, m, &rrt).await?;
        }
    }
    Ok(())
}

/// Remove a specific user's reaction.
async fn react_remove_user(
    http: &Client,
    ch: Id<ChannelMarker>,
    m: Id<MessageMarker>,
    emoji_s: &str,
    user: Id<UserMarker>,
) -> Result<()> {
    match parse_emoji(emoji_s) {
        ReactionEmoji::Unicode(ref name) => {
            let rrt = RequestReactionType::Unicode { name };
            http.delete_reaction(ch, m, &rrt, user).await?;
        }
        ReactionEmoji::Custom { ref name, ref id } => {
            let eid: Id<EmojiMarker> = parse_id(id)?;
            let rrt = RequestReactionType::Custom { id: eid, name: Some(name) };
            http.delete_reaction(ch, m, &rrt, user).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- 5a: limit clamp ---

    #[test]
    fn clamps_limit() {
        assert_eq!(clamp_limit(0), 1);
        assert_eq!(clamp_limit(1), 1);
        assert_eq!(clamp_limit(50), 50);
        assert_eq!(clamp_limit(100), 100);
        assert_eq!(clamp_limit(999), 100);
    }

    // --- 5a: emoji parser ---

    #[test]
    fn parses_unicode_emoji() {
        match parse_emoji("👍") {
            ReactionEmoji::Unicode(s) => assert_eq!(s, "👍"),
            _ => panic!("expected Unicode"),
        }
    }

    #[test]
    fn parses_custom_emoji() {
        match parse_emoji("blob:12345") {
            ReactionEmoji::Custom { id, name } => {
                assert_eq!(id, "12345");
                assert_eq!(name, "blob");
            }
            _ => panic!("expected Custom"),
        }
    }

    #[test]
    fn emoji_with_colon_but_non_digit_id_is_unicode() {
        // "name:text" is not a valid custom emoji (id must be all digits)
        match parse_emoji("name:notanid") {
            ReactionEmoji::Unicode(_) => {}
            _ => panic!("expected Unicode fallback"),
        }
    }

    // --- 5b: aggregate_reactions partial-failure ---

    #[test]
    fn multi_reaction_continues_on_failure_and_reports_each() {
        let results = aggregate_reactions(
            vec!["a".into(), "bad".into(), "c".into()],
            |e| if e == "bad" { Err("nope".into()) } else { Ok(()) },
        );
        assert_eq!(results.iter().filter(|r| r.ok).count(), 2);
        assert_eq!(results.iter().filter(|r| !r.ok).count(), 1);
        let bad = results.iter().find(|r| !r.ok).unwrap();
        assert_eq!(bad.error.as_deref(), Some("nope"));
    }

    #[test]
    fn all_succeed_when_no_failures() {
        let results = aggregate_reactions(
            vec!["a".into(), "b".into(), "c".into()],
            |_| Ok(()),
        );
        assert!(results.iter().all(|r| r.ok));
    }
}
