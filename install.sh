#!/bin/sh
# discord-use installer — downloads a prebuilt binary from GitHub Releases.
# No npm, no Node, no tokens required.
#
#   curl -fsSL https://raw.githubusercontent.com/leeguooooo/discord-use/main/install.sh | sh
#
# Env overrides:
#   DISCORD_USE_VERSION=v0.1.0   install a specific tag (default: latest release)
#   DISCORD_USE_INSTALL_DIR=...  install dir (default: ~/.local/bin)
set -eu

REPO="leeguooooo/discord-use"
INSTALL_DIR="${DISCORD_USE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${DISCORD_USE_VERSION:-}"

err() { printf 'discord-use-install: %s\n' "$1" >&2; exit 1; }

# --- detect platform ---------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Darwin)
    case "$arch" in
      arm64|aarch64) target="aarch64-apple-darwin" ;;
      x86_64)        target="x86_64-apple-darwin" ;;
      *) err "unsupported macOS arch: $arch" ;;
    esac ;;
  Linux)
    case "$arch" in
      x86_64)        target="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) target="aarch64-unknown-linux-gnu" ;;
      *) err "unsupported Linux arch: $arch" ;;
    esac ;;
  *) err "unsupported OS: $os (macOS and Linux only; on Windows use WSL)" ;;
esac

# --- resolve download URL ----------------------------------------------------
asset="discord-use-${target}.tar.gz"
if [ -n "$VERSION" ]; then
  base="https://github.com/${REPO}/releases/download/${VERSION}"
else
  base="https://github.com/${REPO}/releases/latest/download"
fi
url="${base}/${asset}"

# --- download + verify + install ---------------------------------------------
command -v curl >/dev/null 2>&1 || err "curl is required"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

printf 'discord-use-install: downloading %s\n' "$url"
if ! curl -fSL --retry 3 -o "$tmp/$asset" "$url"; then
  # Releases before v0.2.1 have no Intel Mac binary.
  [ "$target" = x86_64-apple-darwin ] && \
    err "download failed: this release has no Intel Mac binary — set DISCORD_USE_VERSION to v0.2.1 or later, or build from source: 'cargo install --git https://github.com/${REPO} --root ~/.local' (needs Rust)"
  err "download failed (does the release have ${asset}?)"
fi

# Optional checksum verification when the .sha256 sidecar is present.
if curl -fsSL --retry 2 -o "$tmp/$asset.sha256" "${url}.sha256" 2>/dev/null; then
  expected="$(awk '{print $1}' "$tmp/$asset.sha256")"
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/$asset" | awk '{print $1}')"
  elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$tmp/$asset" | awk '{print $1}')"
  else
    actual=""
  fi
  if [ -n "$actual" ] && [ "$expected" != "$actual" ]; then
    err "checksum mismatch (expected $expected, got $actual)"
  fi
  [ -n "$actual" ] && printf 'discord-use-install: checksum ok\n'
fi

tar -xzf "$tmp/$asset" -C "$tmp"
[ -f "$tmp/discord-use" ] || err "archive did not contain a 'discord-use' binary"

mkdir -p "$INSTALL_DIR"
# Stage next to the target, then rename: $tmp is often on another filesystem,
# where a plain mv copies over the existing (possibly running) binary in place
# and leaves a truncated file if interrupted. rename(2) within one dir is atomic.
stage="$INSTALL_DIR/.discord-use.new.$$"
trap 'rm -rf "$tmp"; rm -f "$stage"' EXIT
cp "$tmp/discord-use" "$stage"
chmod +x "$stage"
mv -f "$stage" "$INSTALL_DIR/discord-use"

printf 'discord-use-install: installed to %s/discord-use\n' "$INSTALL_DIR"
"$INSTALL_DIR/discord-use" --version >/dev/null 2>&1 && \
  printf 'discord-use-install: version %s\n' "$("$INSTALL_DIR/discord-use" --version 2>/dev/null)" || true

# --- PATH hint ---------------------------------------------------------------
case ":$PATH:" in
  *":$INSTALL_DIR:"*) : ;;
  *) printf 'discord-use-install: NOTE — add %s to your PATH:\n  export PATH="%s:$PATH"\n' "$INSTALL_DIR" "$INSTALL_DIR" ;;
esac
