//! The daily update notice, end to end through the real binary. The cache is
//! seeded fresh so nothing touches the network, and no token is available, so
//! commands fail at token resolution (after the notice) without calling Discord.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const NOTICE_PREFIX: &str = "discord-use 99.0.0 is available (you have ";

fn run(args: &[&str], extra_env: &[(&str, &str)]) -> Output {
    static N: AtomicU32 = AtomicU32::new(0);
    let home: PathBuf = std::env::temp_dir().join(format!(
        "discord-use-notice-test-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    let cache = home.join("cache/discord-use");
    std::fs::create_dir_all(&cache).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(
        cache.join("update-check.json"),
        format!(r#"{{"checked_at": {now}, "latest": "99.0.0"}}"#),
    )
    .unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_discord-use"));
    cmd.args(args)
        .env("HOME", &home)
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env_remove("DISCORD_TOKEN")
        .env_remove("CI")
        .env_remove("DISCORD_USE_NO_UPDATE_CHECK")
        .env_remove("USE_NO_UPDATE_CHECK")
        .stdin(std::process::Stdio::null());
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    let _ = std::fs::remove_dir_all(&home);
    out
}

fn has_notice(out: &Output) -> bool {
    let all = [out.stdout.as_slice(), out.stderr.as_slice()].concat();
    String::from_utf8_lossy(&all).contains("is available")
}

#[test]
fn notice_is_one_stderr_line_and_never_on_stdout() {
    let out = run(&["read", "--channel-id", "1"], &[]);
    assert!(
        !out.status.success(),
        "no token, so the command itself fails"
    );
    assert!(
        out.stdout.is_empty(),
        "{:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    let notices: Vec<_> = stderr
        .lines()
        .filter(|l| l.contains("is available"))
        .collect();
    assert_eq!(notices.len(), 1, "{stderr}");
    assert!(notices[0].starts_with(NOTICE_PREFIX), "{stderr}");
    assert!(
        notices[0].ends_with("). Upgrade: discord-use upgrade"),
        "{stderr}"
    );
}

#[test]
fn mcp_mode_never_checks() {
    let out = run(&["mcp"], &[]);
    assert!(
        !has_notice(&out),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn opt_outs_and_skipped_invocations_print_nothing() {
    for (k, v) in [
        ("CI", "true"),
        ("DISCORD_USE_NO_UPDATE_CHECK", "1"),
        ("USE_NO_UPDATE_CHECK", "1"),
    ] {
        let out = run(&["read", "--channel-id", "1"], &[(k, v)]);
        assert!(!has_notice(&out), "{k}");
    }
    for args in [&["--version"][..], &["--help"], &["upgrade", "--help"]] {
        let out = run(args, &[]);
        assert!(out.status.success(), "{args:?}");
        assert!(!has_notice(&out), "{args:?}");
    }
}
