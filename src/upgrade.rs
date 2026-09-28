//! `discord-use upgrade` and the once-a-day "new version" notice, following
//! the *-use family upgrade convention (leeguooooo/plugins docs/upgrade.md).
//!
//! Nothing here reads the Discord token: the only state is
//! `${XDG_CACHE_HOME:-~/.cache}/discord-use/update-check.json`, and the only
//! network call is the GitHub releases API. `main` skips the notice entirely
//! in `mcp` mode (see there).

use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Context as _;

pub const NAME: &str = "discord-use";
const REPO: &str = "leeguooooo/discord-use";
const PLUGIN: &str = "discord-use@leeguooooo-plugins";
const INSTALL_SCRIPT: &str =
    "https://raw.githubusercontent.com/leeguooooo/discord-use/main/install.sh";
const OPT_OUT_VARS: [&str; 3] = ["CI", "DISCORD_USE_NO_UPDATE_CHECK", "USE_NO_UPDATE_CHECK"];
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const NOTICE_TIMEOUT: Duration = Duration::from_secs(2);
const UPGRADE_TIMEOUT: Duration = Duration::from_secs(15);

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// `X.Y.Z` (optionally `vX.Y.Z`, build/pre-release suffix ignored).
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim();
    let s = s.strip_prefix('v').unwrap_or(s);
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// True only when both parse and `latest` is strictly newer.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// `CI`, `DISCORD_USE_NO_UPDATE_CHECK` or `USE_NO_UPDATE_CHECK` set to a
/// non-empty value disables the check and the notice.
pub fn checks_disabled(get: impl Fn(&str) -> Option<OsString>) -> bool {
    OPT_OUT_VARS
        .iter()
        .any(|k| get(k).is_some_and(|v| !v.is_empty()))
}

fn env_nonempty(key: &str) -> Option<OsString> {
    std::env::var_os(key).filter(|v| !v.is_empty())
}

pub fn cache_file(get: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let base = get("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            get("HOME")
                .filter(|v| !v.is_empty())
                .map(|h| PathBuf::from(h).join(".cache"))
        })?;
    Some(base.join(NAME).join("update-check.json"))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CheckCache {
    pub checked_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
}

fn read_cache(path: &Path) -> Option<CheckCache> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

fn write_cache(path: &Path, cache: &CheckCache) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(cache)?)?;
    std::fs::rename(tmp, path)
}

/// Fresh = checked within the last 24 h. A timestamp in the future (clock
/// moved back) counts as stale so the check cannot get stuck.
pub fn is_fresh(cache: &CheckCache, now: u64) -> bool {
    cache.checked_at <= now && now - cache.checked_at < CHECK_INTERVAL_SECS
}

pub fn notice_line(latest: &str, current: &str) -> Option<String> {
    is_newer(latest, current).then(|| {
        format!(
            "{NAME} {latest} is available (you have {current}). \
             Upgrade: {NAME} upgrade"
        )
    })
}

/// Uses the cache when fresh; otherwise calls `fetch` once and records the
/// attempt (`checked_at` is updated even when `fetch` fails, keeping the
/// previously known `latest`). Returns the notice line, if any.
pub async fn check_with<F, Fut>(
    cache_path: &Path,
    now: u64,
    current: &str,
    fetch: F,
) -> Option<String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<String>>,
{
    let cached = read_cache(cache_path);
    let latest = match cached {
        Some(c) if is_fresh(&c, now) => c.latest,
        stale => {
            let latest = fetch().await.ok().or_else(|| stale.and_then(|c| c.latest));
            let _ = write_cache(
                cache_path,
                &CheckCache {
                    checked_at: now,
                    latest: latest.clone(),
                },
            );
            latest
        }
    };
    notice_line(&latest?, current)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The daily notice. Writes at most one line to `err` (stderr in main).
/// `skip` is true for `upgrade` and `mcp`; `--help`/`--version` never get
/// here because clap exits while parsing.
pub async fn maybe_notify_to<F, Fut>(
    err: &mut impl std::io::Write,
    skip: bool,
    get: impl Fn(&str) -> Option<OsString>,
    now: u64,
    fetch: F,
) where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<String>>,
{
    if skip || checks_disabled(&get) {
        return;
    }
    let Some(path) = cache_file(&get) else {
        return;
    };
    if let Some(line) = check_with(&path, now, current_version(), fetch).await {
        let _ = writeln!(err, "{line}");
    }
}

pub async fn maybe_notify(skip: bool) {
    maybe_notify_to(
        &mut std::io::stderr(),
        skip,
        |k| std::env::var_os(k),
        now_unix(),
        || fetch_latest(NOTICE_TIMEOUT),
    )
    .await;
}

/// Extracts `X.Y.Z` from a `releases/latest` response.
pub fn parse_release(v: &serde_json::Value) -> anyhow::Result<String> {
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) {
        anyhow::bail!("latest release is a draft or prerelease");
    }
    let tag = v["tag_name"].as_str().context("release has no tag_name")?;
    let (a, b, c) =
        parse_version(tag).with_context(|| format!("unexpected release tag {tag:?}"))?;
    Ok(format!("{a}.{b}.{c}"))
}

async fn fetch_latest(timeout: Duration) -> anyhow::Result<String> {
    tokio::time::timeout(timeout, fetch_latest_inner())
        .await
        .map_err(|_| anyhow::anyhow!("timed out after {}s", timeout.as_secs()))?
}

/// GitHub `releases/latest` over the hyper + rustls stack twilight-http
/// already ships, so the check adds no new crates.
async fn fetch_latest_inner() -> anyhow::Result<String> {
    use http_body_util::{BodyExt as _, Empty};
    use hyper::body::Bytes;
    use hyper::header::{ACCEPT, AUTHORIZATION, USER_AGENT};

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .try_with_platform_verifier()?
        .https_only()
        .enable_http1()
        .build();
    let client: hyper_util::client::legacy::Client<_, Empty<Bytes>> =
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build(https);
    let mut req = hyper::Request::get(format!(
        "https://api.github.com/repos/{REPO}/releases/latest"
    ))
    .header(
        USER_AGENT,
        concat!("discord-use/", env!("CARGO_PKG_VERSION")),
    )
    .header(ACCEPT, "application/vnd.github+json");
    if let Some(token) = env_nonempty("GITHUB_TOKEN") {
        req = req.header(AUTHORIZATION, format!("Bearer {}", token.to_string_lossy()));
    }
    let resp = client.request(req.body(Empty::new())?).await?;
    let status = resp.status();
    anyhow::ensure!(status.is_success(), "GitHub API returned {status}");
    let body = resp.into_body().collect().await?.to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body)?;
    parse_release(&v)
}

// ---------------------------------------------------------------- skills

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Skill {
    pub channel: &'static str,
    pub path: String,
    pub update: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub name: &'static str,
    pub current: String,
    pub latest: String,
    pub update_available: bool,
    pub skills: Vec<Skill>,
}

pub fn report(current: &str, latest: &str, skills: Vec<Skill>) -> Report {
    Report {
        name: NAME,
        current: current.to_string(),
        latest: latest.to_string(),
        update_available: is_newer(latest, current),
        skills,
    }
}

pub fn check_line(current: &str, latest: &str) -> String {
    if is_newer(latest, current) {
        format!("{NAME} {current} -> {latest}")
    } else {
        format!("{NAME} {current} is up to date")
    }
}

/// `installPath` of a `<name>@...` entry in `installed_plugins.json`
/// (v1 object or v2 array form), or the file itself when none is recorded.
fn plugin_install(json: &serde_json::Value, file: &Path) -> Option<String> {
    let prefix = format!("{NAME}@");
    let maps = [json.get("plugins"), Some(json)];
    for map in maps.into_iter().flatten() {
        let Some(obj) = map.as_object() else { continue };
        for (key, val) in obj {
            if !key.starts_with(&prefix) {
                continue;
            }
            let entry = val.as_array().and_then(|a| a.first()).unwrap_or(val);
            return Some(
                entry["installPath"]
                    .as_str()
                    .map_or_else(|| file.display().to_string(), str::to_string),
            );
        }
    }
    None
}

fn git_toplevel(dir: &Path) -> Option<PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let top = String::from_utf8(out.stdout).ok()?;
    let top = top.trim();
    (!top.is_empty()).then(|| PathBuf::from(top))
}

/// Every place this skill is installed, per the convention's channel table.
pub fn find_skills(home: &Path) -> Vec<Skill> {
    let mut skills = vec![];
    let plugins_file = home.join(".claude/plugins/installed_plugins.json");
    if let Some(path) = std::fs::read(&plugins_file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .and_then(|v| plugin_install(&v, &plugins_file))
    {
        skills.push(Skill {
            channel: "claude-plugin",
            path,
            update: format!("claude plugin update {PLUGIN}"),
        });
    }
    let plugin_cache = home.join(".claude/plugins");
    let mut seen: Vec<PathBuf> = vec![];
    for dir in [".agents/skills", ".claude/skills", ".codex/skills"] {
        let link = home.join(dir).join(NAME);
        let Ok(resolved) = std::fs::canonicalize(&link) else {
            continue;
        };
        // A link into the plugin cache is the plugin channel's business.
        if std::fs::canonicalize(&plugin_cache).is_ok_and(|p| resolved.starts_with(p)) {
            continue;
        }
        if let Some(root) = git_toplevel(&resolved) {
            if !seen.contains(&root) {
                skills.push(Skill {
                    channel: "git",
                    path: root.display().to_string(),
                    update: format!("git -C {} pull --ff-only", root.display()),
                });
                seen.push(root);
            }
        } else if resolved.is_dir()
            && resolved.join("SKILL.md").is_file()
            && !seen.contains(&resolved)
        {
            skills.push(Skill {
                channel: "copied",
                path: link.display().to_string(),
                update: format!("npx skills update {NAME}"),
            });
            seen.push(resolved);
        }
    }
    skills
}

fn find_on_path(bin: &str, path: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    std::env::split_paths(path?)
        .map(|d| d.join(bin))
        .find(|p| p.is_file())
}

/// Refreshes one skill; returns a one-line human result.
pub fn refresh_skill(skill: &Skill, path_env: Option<&std::ffi::OsStr>) -> String {
    match skill.channel {
        "claude-plugin" => {
            let Some(claude) = find_on_path("claude", path_env) else {
                return format!("run: {}", skill.update);
            };
            match std::process::Command::new(claude)
                .args(["plugin", "update", PLUGIN])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                Ok(s) if s.success() => "updated".to_string(),
                Ok(s) => format!("{} failed ({s}); run it by hand", skill.update),
                Err(e) => format!("could not run claude ({e}): {}", skill.update),
            }
        }
        "git" => match std::process::Command::new("git")
            .args(["-C", &skill.path, "pull", "--ff-only", "--quiet"])
            .stdin(std::process::Stdio::null())
            .output()
        {
            Ok(o) if o.status.success() => "pulled".to_string(),
            Ok(o) => {
                let why = String::from_utf8_lossy(&o.stderr);
                let why = why
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("")
                    .to_string();
                format!("not updated, git pull --ff-only failed: {why}")
            }
            Err(e) => format!("not updated, could not run git: {e}"),
        },
        _ => format!("run: {}", skill.update),
    }
}

// ---------------------------------------------------------------- install

#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    /// Release binary from install.sh (or unpacked by hand) in this dir.
    Installer(PathBuf),
    /// `cargo install` into ~/.cargo/bin.
    Cargo,
    /// A `target/{debug,release}` build from a checkout.
    Source(PathBuf),
}

pub fn install_route(exe: &Path, cargo_bin: Option<&Path>) -> Route {
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let comps: Vec<_> = dir.components().map(|c| c.as_os_str().to_owned()).collect();
    let in_target = comps.windows(2).any(|w| w[0] == "target")
        && comps
            .last()
            .is_some_and(|c| c == "debug" || c == "release" || c == "deps");
    if in_target {
        return Route::Source(exe.to_path_buf());
    }
    if cargo_bin.is_some_and(|c| dir == c) {
        return Route::Cargo;
    }
    Route::Installer(dir.to_path_buf())
}

fn cargo_bin() -> Option<PathBuf> {
    env_nonempty("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env_nonempty("HOME").map(|h| PathBuf::from(h).join(".cargo")))
        .map(|c| c.join("bin"))
        .and_then(|p| std::fs::canonicalize(p).ok())
}

/// Runs the repo's install.sh pinned to `v<latest>` into `dir`. The script
/// is downloaded to a temp file first so a failed download can't be mistaken
/// for a successful empty script.
fn run_installer(dir: &Path, latest: &str) -> anyhow::Result<()> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(
            "set -eu; t=$(mktemp); trap 'rm -f \"$t\"' EXIT; \
             curl -fsSL \"$1\" -o \"$t\"; sh \"$t\"",
        )
        .arg("sh")
        .arg(INSTALL_SCRIPT)
        .env("DISCORD_USE_INSTALL_DIR", dir)
        .env("DISCORD_USE_VERSION", format!("v{latest}"))
        .stdin(std::process::Stdio::null())
        // install.sh reports progress on stderr and ends with a --version
        // line on stdout; keep our stdout for the summary.
        .stdout(std::process::Stdio::null())
        .status()
        .context("could not run sh")?;
    anyhow::ensure!(status.success(), "install.sh failed ({status})");
    Ok(())
}

/// `discord-use upgrade [--check] [--json]`. Needs no Discord token. Returns the exit code:
/// 0 on success (including "already current"), 2 when the check or the
/// download failed.
pub async fn run(check: bool, json: bool) -> i32 {
    let current = current_version();
    let latest = match fetch_latest(UPGRADE_TIMEOUT).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{NAME} upgrade: could not get the latest release: {e:#}");
            return 2;
        }
    };
    if let Some(path) = cache_file(|k| std::env::var_os(k)) {
        let _ = write_cache(
            &path,
            &CheckCache {
                checked_at: now_unix(),
                latest: Some(latest.clone()),
            },
        );
    }
    let home = env_nonempty("HOME").map(PathBuf::from);
    let skills = home.as_deref().map(find_skills).unwrap_or_default();

    if json {
        let r = report(current, &latest, skills);
        match serde_json::to_string_pretty(&r) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("{NAME} upgrade: {e}");
                return 2;
            }
        }
        return 0;
    }
    if check {
        println!("{}", check_line(current, &latest));
        return 0;
    }

    let mut out = std::io::stdout().lock();
    if is_newer(&latest, current) {
        let exe = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .unwrap_or_else(|_| PathBuf::from(NAME));
        match install_route(&exe, cargo_bin().as_deref()) {
            Route::Installer(dir) => {
                if let Err(e) = run_installer(&dir, &latest) {
                    eprintln!("{NAME} upgrade: {e:#}");
                    return 2;
                }
                let _ = writeln!(
                    out,
                    "{NAME} {current} -> {latest} (installed to {})",
                    dir.display()
                );
                let _ = writeln!(
                    out,
                    "note: running `{NAME} mcp` servers keep the old \
                     version until their host restarts them"
                );
            }
            Route::Cargo => {
                let _ = writeln!(
                    out,
                    "{NAME} {current} -> {latest} available; installed \
                     with cargo, run: cargo install --locked --git \
                     https://github.com/{REPO} --tag v{latest}"
                );
            }
            Route::Source(exe) => {
                let _ = writeln!(
                    out,
                    "{NAME} {current} -> {latest} available; {} is a \
                     source build, pull the checkout and rebuild",
                    exe.display()
                );
            }
        }
    } else {
        let _ = writeln!(out, "{}", check_line(current, &latest));
    }
    let path_env = std::env::var_os("PATH");
    for skill in &skills {
        let result = refresh_skill(skill, path_env.as_deref());
        let _ = writeln!(out, "skill ({}) {}: {result}", skill.channel, skill.path);
    }
    if skills.is_empty() {
        let _ = writeln!(out, "skill: no installed copy found to refresh");
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Minimal self-cleaning temp dir (no tempfile dependency).
    struct TempDir(PathBuf);
    impl TempDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn tempdir() -> TempDir {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "discord-use-upgrade-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), OsString::from(v)))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn version_comparison() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.10.0-rc.1"), Some((0, 10, 0)));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("latest"), None);
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("v0.4.1", "0.4.0"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.3.9", "0.4.0"));
        assert!(!is_newer("garbage", "0.4.0"));
    }

    #[test]
    fn opt_out_env_vars() {
        assert!(!checks_disabled(env(&[])));
        assert!(checks_disabled(env(&[("CI", "true")])));
        assert!(checks_disabled(env(&[(
            "DISCORD_USE_NO_UPDATE_CHECK",
            "1"
        )])));
        assert!(checks_disabled(env(&[("USE_NO_UPDATE_CHECK", "1")])));
        assert!(!checks_disabled(env(&[("CI", "")])));
        assert!(!checks_disabled(env(&[("DISCORD_NO_UPDATE_CHECK", "1")])));
    }

    #[test]
    fn cache_location() {
        assert_eq!(
            cache_file(env(&[("HOME", "/h"), ("XDG_CACHE_HOME", "/x")])),
            Some(PathBuf::from("/x/discord-use/update-check.json"))
        );
        assert_eq!(
            cache_file(env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h/.cache/discord-use/update-check.json"))
        );
        assert_eq!(cache_file(env(&[])), None);
    }

    #[tokio::test]
    async fn throttles_to_once_per_day() {
        let dir = tempdir();
        let path = dir.path().join("discord-use/update-check.json");
        let t0 = 1_700_000_000;
        let mut calls = 0;
        let line = check_with(&path, t0, "0.4.0", || {
            calls += 1;
            async { Ok("0.5.0".into()) }
        })
        .await;
        assert_eq!(calls, 1);
        assert_eq!(
            line.as_deref(),
            Some(
                "discord-use 0.5.0 is available (you have 0.4.0). \
                 Upgrade: discord-use upgrade"
            )
        );
        assert_eq!(
            read_cache(&path),
            Some(CheckCache {
                checked_at: t0,
                latest: Some("0.5.0".into())
            })
        );
        // Within 24 h: cache only, notice still shown.
        let line = check_with(&path, t0 + CHECK_INTERVAL_SECS - 1, "0.4.0", || async {
            panic!("fetched while cache is fresh")
        })
        .await;
        assert!(line.is_some());
        // After 24 h: fetched again.
        let mut calls = 0;
        let line = check_with(&path, t0 + CHECK_INTERVAL_SECS, "0.5.0", || {
            calls += 1;
            async { Ok("0.5.0".into()) }
        })
        .await;
        assert_eq!(calls, 1);
        assert_eq!(line, None);
    }

    #[tokio::test]
    async fn failed_check_is_silent_and_still_throttled() {
        let dir = tempdir();
        let path = dir.path().join("c/update-check.json");
        let t0 = 1_700_000_000;
        assert_eq!(
            check_with(&path, t0, "0.4.0", || async { anyhow::bail!("offline") }).await,
            None
        );
        assert_eq!(
            read_cache(&path),
            Some(CheckCache {
                checked_at: t0,
                latest: None
            })
        );
        assert_eq!(
            check_with(&path, t0 + 60, "0.4.0", || async { panic!("retried") }).await,
            None
        );
        // A later failure keeps the last known latest.
        write_cache(
            &path,
            &CheckCache {
                checked_at: t0,
                latest: Some("0.6.0".into()),
            },
        )
        .unwrap();
        let line = check_with(&path, t0 + CHECK_INTERVAL_SECS, "0.4.0", || async {
            anyhow::bail!("offline")
        })
        .await;
        assert!(line.is_some());
        assert_eq!(
            read_cache(&path).unwrap().checked_at,
            t0 + CHECK_INTERVAL_SECS
        );
        // Clock moved backwards: treat as stale.
        assert!(!is_fresh(
            &CheckCache {
                checked_at: t0 + 10,
                latest: None
            },
            t0
        ));
    }

    #[tokio::test]
    async fn notice_goes_only_to_the_given_writer_and_respects_skips() {
        let dir = tempdir();
        let home = dir.path().to_str().unwrap();
        let mut err = vec![];
        maybe_notify_to(&mut err, false, env(&[("HOME", home)]), 1, || async {
            Ok("99.0.0".into())
        })
        .await;
        let text = String::from_utf8(err).unwrap();
        assert_eq!(text.lines().count(), 1);
        assert!(text.starts_with("discord-use 99.0.0 is available (you have "));
        assert!(text.ends_with("). Upgrade: discord-use upgrade\n"));

        for (skip, pairs) in [
            (true, vec![("HOME", home)]),
            (false, vec![("HOME", home), ("CI", "1")]),
            (false, vec![("HOME", home), ("USE_NO_UPDATE_CHECK", "1")]),
            (
                false,
                vec![("HOME", home), ("DISCORD_USE_NO_UPDATE_CHECK", "1")],
            ),
        ] {
            let mut err = vec![];
            maybe_notify_to(&mut err, skip, env(&pairs), 2, || async {
                panic!("checked although skipped")
            })
            .await;
            assert!(err.is_empty());
        }
    }

    #[test]
    fn release_parsing() {
        let v = serde_json::json!({"tag_name": "v0.5.0", "prerelease": false});
        assert_eq!(parse_release(&v).unwrap(), "0.5.0");
        assert!(parse_release(&serde_json::json!({"tag_name": "nightly"})).is_err());
        assert!(
            parse_release(&serde_json::json!({"tag_name": "v1.0.0", "prerelease": true})).is_err()
        );
        assert!(parse_release(&serde_json::json!({})).is_err());
    }

    #[test]
    fn json_report_shape() {
        let r = report(
            "0.4.0",
            "0.5.0",
            vec![Skill {
                channel: "claude-plugin",
                path: "/p".into(),
                update: format!("claude plugin update {PLUGIN}"),
            }],
        );
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "name": "discord-use",
                "current": "0.4.0",
                "latest": "0.5.0",
                "update_available": true,
                "skills": [{
                    "channel": "claude-plugin",
                    "path": "/p",
                    "update": "claude plugin update discord-use@leeguooooo-plugins"
                }]
            })
        );
        let v = serde_json::to_value(report("0.5.0", "0.5.0", vec![])).unwrap();
        assert_eq!(v["update_available"], false);
        assert_eq!(v["skills"], serde_json::json!([]));
        assert_eq!(check_line("0.4.0", "0.5.0"), "discord-use 0.4.0 -> 0.5.0");
        assert_eq!(
            check_line("0.5.0", "0.5.0"),
            "discord-use 0.5.0 is up to date"
        );
    }

    #[test]
    fn skill_channels() {
        let dir = tempdir();
        let home = dir.path().canonicalize().unwrap();
        assert!(find_skills(&home).is_empty());

        // Claude Code plugin (v2 format).
        std::fs::create_dir_all(home.join(".claude/plugins")).unwrap();
        std::fs::write(
            home.join(".claude/plugins/installed_plugins.json"),
            r#"{"version":2,"plugins":{"discord-use@leeguooooo-plugins":
                [{"scope":"user","installPath":"/cache/du"}],
                "other@x":[{}]}}"#,
        )
        .unwrap();
        // Copied folder.
        let copied = home.join(".agents/skills/discord-use");
        std::fs::create_dir_all(&copied).unwrap();
        std::fs::write(copied.join("SKILL.md"), "x").unwrap();
        // Git checkout (symlinked, as a clone-based install would be).
        let repo = home.join("src/discord-use");
        std::fs::create_dir_all(&repo).unwrap();
        let ok = std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(&repo)
            .status()
            .is_ok_and(|s| s.success());
        std::fs::create_dir_all(home.join(".claude/skills")).unwrap();
        std::os::unix::fs::symlink(&repo, home.join(".claude/skills/discord-use")).unwrap();
        // A link into the plugin cache is not reported twice.
        std::fs::create_dir_all(home.join(".claude/plugins/cache/b")).unwrap();
        std::fs::create_dir_all(home.join(".codex/skills")).unwrap();
        std::os::unix::fs::symlink(
            home.join(".claude/plugins/cache/b"),
            home.join(".codex/skills/discord-use"),
        )
        .unwrap();

        let skills = find_skills(&home);
        let channels: Vec<_> = skills.iter().map(|s| s.channel).collect();
        if ok {
            assert_eq!(channels, ["claude-plugin", "copied", "git"]);
            assert_eq!(skills[2].path, repo.display().to_string());
            // No remote: pull fails, is reported, nothing is forced.
            assert!(refresh_skill(&skills[2], None).starts_with("not updated"));
        } else {
            assert_eq!(channels, ["claude-plugin", "copied", "copied"]);
        }
        assert_eq!(skills[0].path, "/cache/du");
        assert_eq!(skills[1].update, "npx skills update discord-use");
        // `claude` not on PATH: print the command, run nothing.
        let empty = home.join("empty-bin");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(
            refresh_skill(&skills[0], Some(empty.as_os_str())),
            "run: claude plugin update discord-use@leeguooooo-plugins"
        );
        assert_eq!(
            refresh_skill(&skills[1], None),
            "run: npx skills update discord-use"
        );
    }

    #[test]
    fn install_routes() {
        let cargo = Path::new("/home/u/.cargo/bin");
        assert_eq!(
            install_route(Path::new("/home/u/.local/bin/discord-use"), Some(cargo)),
            Route::Installer(PathBuf::from("/home/u/.local/bin"))
        );
        assert_eq!(
            install_route(Path::new("/home/u/.cargo/bin/discord-use"), Some(cargo)),
            Route::Cargo
        );
        assert_eq!(
            install_route(Path::new("/src/du/target/release/discord-use"), Some(cargo)),
            Route::Source(PathBuf::from("/src/du/target/release/discord-use"))
        );
    }
}
