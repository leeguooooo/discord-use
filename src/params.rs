use serde::Deserialize;
use schemars::JsonSchema;

macro_rules! params {
    ($name:ident { $($body:tt)* }) => {
        #[derive(Debug, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($body)* }
    };
}

params!(LoginParams { pub token: Option<String> });
params!(GetServerInfoParams { pub guild_id: String });
params!(SendParams { pub channel_id: String, pub message: String });
params!(ReadMessagesParams {
    pub channel_id: String,
    #[serde(default = "default_limit")] pub limit: u16,   // default 50, clamp 1..=100 in discord.rs
});
fn default_limit() -> u16 { 50 }
params!(DeleteMessageParams { pub channel_id: String, pub message_id: String, pub reason: Option<String> });
params!(AddReactionParams { pub channel_id: String, pub message_id: String, pub emoji: String });
params!(AddMultipleReactionsParams { pub channel_id: String, pub message_id: String, pub emojis: Vec<String> });
params!(RemoveReactionParams { pub channel_id: String, pub message_id: String, pub emoji: String, pub user_id: Option<String> });
params!(CreateTextChannelParams { pub guild_id: String, pub channel_name: String, pub topic: Option<String> });
params!(DeleteChannelParams { pub channel_id: String, pub reason: Option<String> });
params!(CreateCategoryParams { pub guild_id: String, pub name: String, pub position: Option<u16>, pub reason: Option<String> });
params!(EditCategoryParams { pub category_id: String, pub name: Option<String>, pub position: Option<u16>, pub reason: Option<String> });
params!(DeleteCategoryParams { pub category_id: String, pub reason: Option<String> });
params!(CreateWebhookParams { pub channel_id: String, pub name: String, pub avatar: Option<String>, pub reason: Option<String> });
params!(EditWebhookParams { pub webhook_id: String, pub name: Option<String>, pub channel_id: Option<String>, pub avatar: Option<String>, pub webhook_token: Option<String>, pub reason: Option<String> });
params!(DeleteWebhookParams { pub webhook_id: String, pub webhook_token: Option<String>, pub reason: Option<String> });

// QUIRK: original uses `avatarURL`, not camelCase `avatarUrl`. Explicit rename.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendWebhookMessageParams {
    pub webhook_id: String,
    pub webhook_token: String,
    pub content: String,
    pub username: Option<String>,
    #[serde(rename = "avatarURL")] pub avatar_url: Option<String>,
    pub thread_id: Option<String>,
}

params!(GetForumChannelsParams { pub guild_id: String });
params!(CreateForumPostParams { pub forum_channel_id: String, pub title: String, pub content: String, pub tags: Option<Vec<String>> });
params!(GetForumPostParams { pub thread_id: String });
params!(ReplyToForumParams { pub thread_id: String, pub message: String });
params!(DeleteForumPostParams { pub thread_id: String, pub reason: Option<String> });

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;

    fn prop_names(schema: schemars::schema::RootSchema) -> Vec<String> {
        schema.schema.object.map(|o| o.properties.keys().cloned().collect()).unwrap_or_default()
    }

    #[test]
    fn send_uses_channelId_and_message() {
        let names = prop_names(schema_for!(SendParams));
        assert!(names.contains(&"channelId".to_string()));
        assert!(names.contains(&"message".to_string()));
        assert!(!names.contains(&"channel_id".to_string()));
    }

    #[test]
    fn webhook_message_uses_avatarURL_quirk() {
        // exhaustive check: all 6 properties must match the drop-in contract exactly
        assert_props(
            prop_names(schema_for!(SendWebhookMessageParams)),
            &["webhookId", "webhookToken", "content", "username", "avatarURL", "threadId"],
        );
    }

    #[test]
    fn create_text_channel_uses_channelName() {
        let names = prop_names(schema_for!(CreateTextChannelParams));
        assert!(names.contains(&"channelName".to_string()));
        assert!(names.contains(&"guildId".to_string()));
    }

    // Table-driven guard over the remaining quirk-adjacent structs. The casing contract is
    // THE correctness guard, so assert exact property-name sets, not just the obvious ones.
    fn assert_props(actual: Vec<String>, expected: &[&str]) {
        let mut a = actual.clone(); a.sort();
        let mut e: Vec<String> = expected.iter().map(|s| s.to_string()).collect(); e.sort();
        assert_eq!(a, e, "schema property names must match the drop-in contract exactly");
    }

    #[test]
    fn remaining_structs_match_contract() {
        assert_props(prop_names(schema_for!(GetServerInfoParams)), &["guildId"]);
        assert_props(prop_names(schema_for!(ReadMessagesParams)), &["channelId", "limit"]);
        assert_props(prop_names(schema_for!(DeleteMessageParams)), &["channelId", "messageId", "reason"]);
        assert_props(prop_names(schema_for!(AddReactionParams)), &["channelId", "messageId", "emoji"]);
        assert_props(prop_names(schema_for!(AddMultipleReactionsParams)), &["channelId", "messageId", "emojis"]);
        assert_props(prop_names(schema_for!(RemoveReactionParams)), &["channelId", "messageId", "emoji", "userId"]);
        assert_props(prop_names(schema_for!(DeleteChannelParams)), &["channelId", "reason"]);
        assert_props(prop_names(schema_for!(CreateCategoryParams)), &["guildId", "name", "position", "reason"]);
        assert_props(prop_names(schema_for!(EditCategoryParams)), &["categoryId", "name", "position", "reason"]);
        assert_props(prop_names(schema_for!(DeleteCategoryParams)), &["categoryId", "reason"]);
        assert_props(prop_names(schema_for!(CreateWebhookParams)), &["channelId", "name", "avatar", "reason"]);
        assert_props(prop_names(schema_for!(EditWebhookParams)), &["webhookId", "name", "channelId", "avatar", "webhookToken", "reason"]);
        assert_props(prop_names(schema_for!(DeleteWebhookParams)), &["webhookId", "webhookToken", "reason"]);
        assert_props(prop_names(schema_for!(GetForumChannelsParams)), &["guildId"]);
        assert_props(prop_names(schema_for!(CreateForumPostParams)), &["forumChannelId", "title", "content", "tags"]);
        assert_props(prop_names(schema_for!(GetForumPostParams)), &["threadId"]);
        assert_props(prop_names(schema_for!(ReplyToForumParams)), &["threadId", "message"]);
        assert_props(prop_names(schema_for!(DeleteForumPostParams)), &["threadId", "reason"]);
        assert_props(prop_names(schema_for!(LoginParams)), &["token"]);
    }
}
