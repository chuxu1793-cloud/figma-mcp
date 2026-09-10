#!/bin/bash
# Prepares the figma-mcp server + Figma plugin entirely in place inside this
# skill's folder: the binary runs from SKILL_DIR/bin/<platform asset> and is
# never copied elsewhere — MCP configs point directly at it, and the plugin is
# unpacked to SKILL_DIR/plugin/.
#   default:        verify the bundled binary offline against bin/SHA256SUMS.txt,
#                   mark it executable, strip macOS quarantine, unpack the plugin
#   --version <tag>: download that GitHub release into bin/ in place, also
#                   refreshing bin/SHA256SUMS.txt and bin/VERSION so later
#                   offline runs verify against the new checksums
# Idempotent. Outputs machine-readable KEY: value lines only.
# Usage: install.sh [--version <tag>] [--force]
set -uo pipefail

REPO="chuxu1793-cloud/figma-mcp"
TAG=""
FORCE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --version) TAG="$2"; shift 2 ;;
    --force)   FORCE=1; shift ;;
    *) echo "STATUS: error"; echo "REASON: unknown argument $1"; exit 2 ;;
  esac
done

fail() { echo "STATUS: error"; echo "REASON: $1"; exit 1; }

SELF_DIR=$(cd "$(dirname "$0")" && pwd)
SKILL_DIR=$(cd "$SELF_DIR/.." && pwd)
BUNDLE_DIR="$SKILL_DIR/bin"
PLUGIN_DIR="$SKILL_DIR/plugin"
BUNDLED_TAG=""
[ -f "$BUNDLE_DIR/VERSION" ] && BUNDLED_TAG=$(tr -d '[:space:]' < "$BUNDLE_DIR/VERSION")

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
# Git Bash / MSYS2 / Cygwin report mingw64_nt-*, msys_nt-*, cygwin_nt-*
case "$OS" in mingw*|msys*|cygwin*) OS="windows" ;; esac

case "$OS-$ARCH" in
  darwin-arm64)   ASSET="figma-mcp-darwin-arm64" ;;
  darwin-x86_64)  ASSET="figma-mcp-darwin-amd64" ;;
  linux-x86_64)   ASSET="figma-mcp-linux-amd64" ;;
  windows-x86_64) ASSET="figma-mcp-windows-amd64.exe" ;;
  *) fail "unsupported platform $OS-$ARCH; prebuilt targets are darwin-arm64, darwin-amd64, linux-amd64, windows-amd64" ;;
esac

BIN="$BUNDLE_DIR/$ASSET"
MANIFEST="$PLUGIN_DIR/manifest.json"

# --- source resolution ---------------------------------------------------------
# Bundled by default; an explicit --version (including "latest") always goes
# to GitHub; a missing bundled asset for this platform falls back to GitHub.
MODE="bundled"
if [ -n "$TAG" ]; then
  MODE="github"
elif [ ! -f "$BIN" ] || [ ! -f "$BUNDLE_DIR/figma-plugin.zip" ]; then
  MODE="github"
  echo "NOTE: no bundled asset for $OS-$ARCH under $BUNDLE_DIR — falling back to GitHub releases (online)"
fi

TMP=""
if [ "$MODE" = "github" ]; then
  if [ -z "$TAG" ] || [ "$TAG" = "latest" ]; then
    BASE="https://github.com/$REPO/releases/latest/download"
  else
    BASE="https://github.com/$REPO/releases/download/$TAG"
  fi
  command -v curl >/dev/null || fail "curl not found (required for the GitHub download path)"
  TMP=$(mktemp -d) || fail "cannot create temp dir"
fi
trap 'rm -rf "${TMP:-}"' EXIT

if command -v shasum >/dev/null; then SHA="shasum -a 256"
elif command -v sha256sum >/dev/null; then SHA="sha256sum"
else SHA=""; fi

# Git Bash ships no unzip; fall back to PowerShell's Expand-Archive.
extract() {
  if command -v unzip >/dev/null; then
    unzip -qo "$1" -d "$2"
  elif command -v powershell.exe >/dev/null && command -v cygpath >/dev/null; then
    powershell.exe -NoProfile -Command \
      "Expand-Archive -Force -LiteralPath '$(cygpath -w "$1")' -DestinationPath '$(cygpath -w "$2")'" >/dev/null
  else
    return 1
  fi
}

# Expected checksums: the bundled sums file offline, or the release's own copy
# when downloading. Used both to verify the source asset and to decide whether
# the in-place binary is already up to date.
SUMS=""
if [ -n "$SHA" ]; then
  if [ "$MODE" = "bundled" ] && [ -f "$BUNDLE_DIR/SHA256SUMS.txt" ]; then
    SUMS="$BUNDLE_DIR/SHA256SUMS.txt"
  elif [ "$MODE" = "github" ] && curl -fsSL --max-time 30 -o "$TMP/SHA256SUMS.txt" "$BASE/SHA256SUMS.txt"; then
    SUMS="$TMP/SHA256SUMS.txt"
  fi
fi
WANT=""
WANT_ZIP=""
if [ -n "$SUMS" ]; then
  WANT=$(awk -v a="$ASSET" '$2 == a || $2 == "*"a {print $1}' "$SUMS" | head -1)
  WANT_ZIP=$(awk '$2 == "figma-plugin.zip" || $2 == "*figma-plugin.zip" {print $1}' "$SUMS" | head -1)
fi

HAVE=""
if [ -n "$SHA" ] && [ -f "$BIN" ]; then
  HAVE=$($SHA "$BIN" | awk '{print $1}')
fi

if [ "$FORCE" -eq 0 ] && [ -n "$WANT" ] && [ "$HAVE" = "$WANT" ] && [ -x "$BIN" ] && [ -f "$MANIFEST" ]; then
  # Self-healing no-ops for archives / skill installers that lose the
  # executable bit or leave a quarantine attribute behind.
  chmod +x "$BIN" 2>/dev/null
  [ "$OS" = "darwin" ] && xattr -d com.apple.quarantine "$BIN" 2>/dev/null
  echo "STATUS: already-current"
else
  if [ "$MODE" = "bundled" ]; then
    [ -f "$BIN" ] || fail "bundled binary missing: $BIN"
    SRC_BIN="$BIN"
    SRC_ZIP="$BUNDLE_DIR/figma-plugin.zip"
  else
    curl -fsSL --max-time 300 -o "$TMP/$ASSET" "$BASE/$ASSET" || fail "download failed: $BASE/$ASSET"
    curl -fsSL --max-time 300 -o "$TMP/figma-plugin.zip" "$BASE/figma-plugin.zip" || fail "download failed: $BASE/figma-plugin.zip"
    SRC_BIN="$TMP/$ASSET"
    SRC_ZIP="$TMP/figma-plugin.zip"
  fi

  # Verify the source before touching anything: a corrupted bundle or a
  # truncated download must not silently produce a broken install.
  CHECKSUM_STATE="skipped (no SHA256SUMS.txt or hashing tool)"
  if [ -n "$WANT" ]; then
    GOT=$($SHA "$SRC_BIN" | awk '{print $1}')
    [ "$GOT" = "$WANT" ] || fail "checksum mismatch for $ASSET (expected $WANT, got $GOT)"
    CHECKSUM_STATE="verified"
  fi
  if [ -n "$WANT_ZIP" ]; then
    GOT_ZIP=$($SHA "$SRC_ZIP" | awk '{print $1}')
    [ "$GOT_ZIP" = "$WANT_ZIP" ] || fail "checksum mismatch for figma-plugin.zip (expected $WANT_ZIP, got $GOT_ZIP)"
  fi

  if [ "$MODE" = "github" ]; then
    # In-place upgrade: replace the bundled asset and refresh the bundle
    # metadata, so later offline runs verify against the new checksums
    # instead of downgrading back to the old bundled release.
    install -m 755 "$SRC_BIN" "$BIN" || fail "cannot write $BIN"
    if [ -n "$SUMS" ]; then
      install -m 644 "$SUMS" "$BUNDLE_DIR/SHA256SUMS.txt" || true
    fi
    RESOLVED_TAG="$TAG"
    if [ -z "$RESOLVED_TAG" ] || [ "$RESOLVED_TAG" = "latest" ]; then
      RESOLVED_TAG=$(curl -fsSI --max-time 20 "$BASE/SHA256SUMS.txt" 2>/dev/null |
        tr -d '\r' | sed -n 's/^[Ll]ocation: .*\/releases\/download\/\([^/]*\)\/.*/\1/p' | head -1)
    fi
    [ -n "$RESOLVED_TAG" ] && printf '%s\n' "$RESOLVED_TAG" > "$BUNDLE_DIR/VERSION"
  else
    chmod +x "$BIN" || fail "cannot mark $BIN executable"
  fi

  rm -rf "$PLUGIN_DIR"
  extract "$SRC_ZIP" "$SKILL_DIR" || fail "cannot extract plugin into $SKILL_DIR (need unzip, or PowerShell on Windows)"
  [ -f "$MANIFEST" ] || fail "plugin manifest missing after extract: $MANIFEST"

  # macOS quarantines downloaded binaries; strip it so Gatekeeper does not
  # block the unsigned binary when the MCP client spawns it.
  if [ "$OS" = "darwin" ]; then
    xattr -d com.apple.quarantine "$BIN" 2>/dev/null
    xattr -cr "$PLUGIN_DIR" 2>/dev/null
  fi

  echo "STATUS: installed"
  echo "CHECKSUM: $CHECKSUM_STATE"
fi

[ -x "$BIN" ] || fail "binary missing or not executable: $BIN"
[ -f "$MANIFEST" ] || fail "plugin manifest missing: $MANIFEST"

echo "BINARY: $BIN"
echo "MANIFEST: $MANIFEST"
echo "PLUGIN_DIR: $PLUGIN_DIR"
echo "PLATFORM: $OS"
echo "ASSET: $ASSET"
if [ "$MODE" = "bundled" ]; then
  echo "SOURCE: bundled ${BUNDLED_TAG:-unknown} (offline, in place)"
else
  echo "SOURCE: $BASE (installed into bin/ in place)"
fi

# Writing the binary does not stop servers already running: they keep the old
# image and the port, so tool calls would still be served by the old version.
if [ -x "$SELF_DIR/cleanup.sh" ]; then
  STALE_N=$(bash "$SELF_DIR/cleanup.sh" 2>/dev/null |
    sed -n 's/^FOUND: [0-9]* process(es), \([0-9]*\) stale$/\1/p')
  if [ -n "${STALE_N:-}" ] && [ "$STALE_N" -gt 0 ]; then
    echo "STALE: $STALE_N running process(es) still use an outdated binary image"
    echo "NEXT: run this skill's scripts/cleanup.sh --apply"
  fi
fi
