//! Gated live integration tests — all marked `#[ignore]` so `cargo test` stays green.
//!
//! Run against real Discord:
//!   DISCORD_TEST_TOKEN=<bot_token> \
//!   DISCORD_TEST_CHANNEL=<channel_id> \
//!   cargo test --test live -- --ignored --nocapture
//!
//! Optional env vars:
//!   DISCORD_TEST_GUILD           — enables get_server_info test
//!   DISCORD_TEST_WEBHOOK_ID      — enables webhook test (also needs TOKEN below)
//!   DISCORD_TEST_WEBHOOK_TOKEN   — webhook token matching the above ID
//!
//! None of these tests perform destructive operations (no delete_*/create channel/category).
//! TODO: add a destructive suite gated on DISCORD_TEST_THROWAWAY_GUILD for full coverage.

use discord_use::discord::DiscordClient;
use discord_use::params::*;

/// Read a required env var, or return early with a skip message.
macro_rules! require_env {
    ($var:literal) => {
        match std::env::var($var) {
            Ok(v) if !v.is_empty() => v,
            _ => {
                eprintln!("[live] SKIP: {} not set", $var);
                return;
            }
        }
    };
}

/// Read an optional env var — returns None when absent/empty.
fn opt_env(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.is_empty())
}

fn make_client(token: &str) -> DiscordClient {
    // Install the ring provider; the Err case means it's already installed — harmless.
    let _ = rustls::crypto::ring::default_provider().install_default();
    DiscordClient::new(token.to_string())
}

// ---------------------------------------------------------------------------
// login
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn live_login() {
    let token = require_env!("DISCORD_TEST_TOKEN");
    let client = make_client(&token);
    let v = client.login(LoginParams { token: None }).await
        .expect("login should succeed");
    eprintln!("[live] login → {v}");
    assert!(v["loggedIn"].as_bool().unwrap_or(false), "loggedIn must be true");
    assert!(!v["id"].as_str().unwrap_or("").is_empty(), "must return an id");
    println!("login OK: id={} username={}", v["id"], v["username"]);
}

// ---------------------------------------------------------------------------
// send + read_messages
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn live_send_and_read() {
    let token   = require_env!("DISCORD_TEST_TOKEN");
    let channel = require_env!("DISCORD_TEST_CHANNEL");
    let client  = make_client(&token);

    let content = format!("discord-use live test {}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());

    let sent = client.send(SendParams {
        channel_id: channel.clone(),
        message: content.clone(),
    }).await.expect("send should succeed");
    eprintln!("[live] send → {sent}");
    let msg_id = sent["id"].as_str().expect("id in send response").to_string();
    println!("send OK: id={msg_id}");

    let read = client.read_messages(ReadMessagesParams {
        channel_id: channel.clone(),
        limit: 10,
    }).await.expect("read_messages should succeed");
    eprintln!("[live] read_messages → {} messages", read["messages"].as_array().map(|a| a.len()).unwrap_or(0));

    let messages = read["messages"].as_array().expect("messages array");
    let found = messages.iter().any(|m| m["id"].as_str() == Some(&msg_id));
    assert!(found, "sent message id={msg_id} must appear in read_messages result");
    println!("read_messages OK: found sent message");
}

// ---------------------------------------------------------------------------
// add_reaction + remove_reaction
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn live_reactions() {
    let token   = require_env!("DISCORD_TEST_TOKEN");
    let channel = require_env!("DISCORD_TEST_CHANNEL");
    let client  = make_client(&token);

    // Send a throwaway message to react to.
    let sent = client.send(SendParams {
        channel_id: channel.clone(),
        message: "discord-use reaction test (will be reacted to)".to_string(),
    }).await.expect("send should succeed");
    let msg_id = sent["id"].as_str().expect("id").to_string();

    let added = client.add_reaction(AddReactionParams {
        channel_id: channel.clone(),
        message_id: msg_id.clone(),
        emoji: "👍".to_string(),
    }).await.expect("add_reaction should succeed");
    eprintln!("[live] add_reaction → {added}");
    assert_eq!(added["added"].as_bool(), Some(true));
    println!("add_reaction OK");

    let removed = client.remove_reaction(RemoveReactionParams {
        channel_id: channel.clone(),
        message_id: msg_id.clone(),
        emoji: "👍".to_string(),
        user_id: None, // removes bot's own reaction
    }).await.expect("remove_reaction should succeed");
    eprintln!("[live] remove_reaction → {removed}");
    assert_eq!(removed["removed"].as_bool(), Some(true));
    println!("remove_reaction OK");
}

// ---------------------------------------------------------------------------
// get_server_info (optional — needs DISCORD_TEST_GUILD)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn live_server_info() {
    let token = require_env!("DISCORD_TEST_TOKEN");
    let guild = match opt_env("DISCORD_TEST_GUILD") {
        Some(g) => g,
        None => {
            eprintln!("[live] SKIP: DISCORD_TEST_GUILD not set");
            return;
        }
    };
    let client = make_client(&token);
    let v = client.get_server_info(GetServerInfoParams { guild_id: guild.clone() }).await
        .expect("get_server_info should succeed");
    eprintln!("[live] get_server_info → id={} name={} channels={}", v["id"], v["name"],
        v["channels"].as_array().map(|a| a.len()).unwrap_or(0));
    assert!(!v["id"].as_str().unwrap_or("").is_empty());
    println!("get_server_info OK: guild={} name={}", guild, v["name"]);
}

// ---------------------------------------------------------------------------
// send_webhook_message (optional — needs DISCORD_TEST_WEBHOOK_ID + TOKEN)
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore]
async fn live_webhook_send() {
    let token         = require_env!("DISCORD_TEST_TOKEN");
    let webhook_id    = match opt_env("DISCORD_TEST_WEBHOOK_ID") {
        Some(v) => v,
        None => { eprintln!("[live] SKIP: DISCORD_TEST_WEBHOOK_ID not set"); return; }
    };
    let webhook_token = match opt_env("DISCORD_TEST_WEBHOOK_TOKEN") {
        Some(v) => v,
        None => { eprintln!("[live] SKIP: DISCORD_TEST_WEBHOOK_TOKEN not set"); return; }
    };
    let client = make_client(&token);
    let v = client.send_webhook_message(SendWebhookMessageParams {
        webhook_id:    webhook_id.clone(),
        webhook_token: webhook_token.clone(),
        content:       "discord-use webhook live test".to_string(),
        username:      Some("discord-use-test".to_string()),
        avatar_url:    None,
        thread_id:     None,
    }).await.expect("send_webhook_message should succeed");
    eprintln!("[live] send_webhook_message → {v}");
    assert_eq!(v["sent"].as_bool(), Some(true));
    println!("send_webhook_message OK: webhookId={webhook_id}");
}
