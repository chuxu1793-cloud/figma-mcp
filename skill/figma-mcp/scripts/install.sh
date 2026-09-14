#!/bin/bash
# Prepares the figma-mcp server + Figma plugin:
#   * server binary: runs IN PLACE from SKILL_DIR/bin/<platform asset>; MCP
#     configs point directly at it and it is never copied elsewhere
#   * Figma plugin: deploys OUT of the skill folder to a user-visible
#     directory (--plugin-dir, default $HOME/figma, layout <dir>/plugin/) —
#     it is the one path a human must navigate, in Figma's import file picker
#   default:         verify the bundled binary offline against bin/SHA256SUMS.txt,
#                    mark it executable, strip macOS quarantine, deploy the plugin
#   --version <tag>: download that GitHub release into bin/ in place (assets
#                    resume across interrupted transfers), also refreshing
#                    bin/SHA256SUMS.txt, bin/figma-plugin.zip and bin/VERSION
#                    so later offline runs verify against the new checksums
#   --all-platforms: with --version, refresh every platform's binary in bin/,
#                    not just the current one, so the whole bundle stays
#                    consistent with the refreshed SHA256SUMS.txt
# Idempotent. Outputs machine-readable KEY: value lines only.
# Usage: install.sh [--plugin-dir <path>] [--version <tag>] [--all-platforms] [--force]
set -uo pipefail

REPO="chuxu1793-cloud/figma-mcp"
TAG=""
FORCE=0
ALL_PLATFORMS=0
PLUGIN_DIR="$HOME/figma"

while [ $# -gt 0 ]; do
  case "$1" in
    --plugin-dir)    PLUGIN_DIR="$2"; shift 2 ;;
    --version)       TAG="$2"; shift 2 ;;
    --all-platforms) ALL_PLATFORMS=1; shift ;;
    --force)         FORCE=1; shift ;;
    *) echo "STATUS: error"; echo "REASON: unknown argument $1"; exit 2 ;;
  esac
done

fail() { echo "STATUS: error"; echo "REASON: $1"; exit 1; }

SELF_DIR=$(cd "$(dirname "$0")" && pwd)
SKILL_DIR=$(cd "$SELF_DIR/.." && pwd)
BUNDLE_DIR="$SKILL_DIR/bin"
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

# Every release asset name; --all-platforms refreshes all of them in bin/.
ALL_ASSETS="figma-mcp-darwin-arm64 figma-mcp-darwin-amd64 figma-mcp-linux-amd64 figma-mcp-windows-amd64.exe"

BIN="$BUNDLE_DIR/$ASSET"

# The plugin deploys into a user-chosen directory (default $HOME/figma). Guard:
# <plugin-dir>/plugin is deleted before each deploy, so refuse root-ish paths
# where that would be destructive.
PLUGIN_DIR="${PLUGIN_DIR%/}"
case "$PLUGIN_DIR" in
  ""|"/"|"$HOME") fail "refusing to deploy the plugin directly into '${PLUGIN_DIR:-/}'; pass a dedicated --plugin-dir such as \$HOME/figma" ;;
esac
MANIFEST="$PLUGIN_DIR/plugin/manifest.json"

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
if [ "$ALL_PLATFORMS" -eq 1 ] && [ "$MODE" = "bundled" ]; then
  echo "NOTE: --all-platforms applies only to --version downloads; the bundled bin/ already ships every platform"
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

# Resumable asset download: a slow link cannot move an 11–14 MB binary inside
# one --max-time window, so retry with `curl -C -` until the file is whole or
# the attempts run out. curl exit 33 (range not satisfiable) means the file is
# already complete; with -f, resuming a finished file answers 416 (exit 22) —
# either way the expected sha256 decides whether it is genuinely whole.
fetch_asset() { # $1=asset-name  $2=dest  $3=expected-sha256 ("" when unknown)
  local asset="$1" dest="$2" want="${3:-}" rc tries=0 got
  while :; do
    rc=0
    curl -fsSL -C - --max-time 300 -o "$dest" "$BASE/$asset" || rc=$?
    [ "$rc" -eq 0 ] && return 0
    if [ "$rc" -eq 22 ] || [ "$rc" -eq 33 ]; then
      [ -n "$want" ] && [ -n "$SHA" ] || return 1
      got=$($SHA "$dest" 2>/dev/null | awk '{print $1}')
      [ "$got" = "$want" ] && return 0
    fi
    tries=$((tries + 1))
    [ "$tries" -ge 10 ] && return 1
    sleep 2
  done
}

# Checksum of an asset name as listed in $SUMS ("" when unknown).
want_for() {
  [ -n "$SUMS" ] || return 1
  awk -v a="$1" '$2 == a || $2 == "*"a {print $1}' "$SUMS" | head -1
}

# Current sha256 of a file ("" when no hashing tool or the file is absent).
have_for() {
  [ -n "$SHA" ] || return 1
  [ -f "$1" ] || return 1
  $SHA "$1" | awk '{print $1}'
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
WANT=$(want_for "$ASSET")
WANT_ZIP=$(want_for figma-plugin.zip)

HAVE=$(have_for "$BIN")

# "Already current" means the whole bundle agrees with the checksums: the
# current binary, the plugin zip, and with --all-platforms every other
# platform's binary too. A stale zip would otherwise leave a later offline
# run deploying an old plugin against the new SHA256SUMS.txt.
ALL_CURRENT=1
ZW=$(want_for figma-plugin.zip)
[ -n "$ZW" ] && [ "$(have_for "$BUNDLE_DIR/figma-plugin.zip")" = "$ZW" ] || ALL_CURRENT=0
if [ "$ALL_PLATFORMS" -eq 1 ]; then
  for other in $ALL_ASSETS; do
    [ "$other" = "$ASSET" ] && continue
    OW=$(want_for "$other"); OH=$(have_for "$BUNDLE_DIR/$other")
    if [ -z "$OW" ] || [ "$OH" != "$OW" ]; then ALL_CURRENT=0; break; fi
  done
fi

if [ "$FORCE" -eq 0 ] && [ "$ALL_CURRENT" -eq 1 ] && [ -n "$WANT" ] && [ "$HAVE" = "$WANT" ] && [ -x "$BIN" ] && [ -f "$MANIFEST" ]; then
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
    fetch_asset "$ASSET" "$TMP/$ASSET" "$WANT" || fail "download failed: $BASE/$ASSET"
    fetch_asset figma-plugin.zip "$TMP/figma-plugin.zip" "$WANT_ZIP" || fail "download failed: $BASE/figma-plugin.zip"
    if [ "$ALL_PLATFORMS" -eq 1 ]; then
      for other in $ALL_ASSETS; do
        [ "$other" = "$ASSET" ] && continue
        OW=$(want_for "$other")
        # Skip a platform whose bin/ copy already matches the release: the
        # point of --all-platforms is a consistent bundle, not re-fetching
        # what already agrees with SHA256SUMS.txt.
        [ -n "$OW" ] && [ "$(have_for "$BUNDLE_DIR/$other")" = "$OW" ] && continue
        fetch_asset "$other" "$TMP/$other" "$OW" || fail "download failed: $BASE/$other"
      done
    fi
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
  if [ "$ALL_PLATFORMS" -eq 1 ] && [ "$MODE" = "github" ]; then
    for other in $ALL_ASSETS; do
      [ "$other" = "$ASSET" ] && continue
      [ -f "$TMP/$other" ] || continue   # skipped above: bin/ already matches
      OW=$(want_for "$other")
      [ -n "$OW" ] || fail "SHA256SUMS.txt has no checksum for $other"
      OG=$($SHA "$TMP/$other" | awk '{print $1}')
      [ "$OG" = "$OW" ] || fail "checksum mismatch for $other (expected $OW, got $OG)"
    done
  fi

  if [ "$MODE" = "github" ]; then
    # In-place upgrade: replace the bundled asset and refresh the bundle
    # metadata, so later offline runs verify against the new checksums
    # instead of downgrading back to the old bundled release.
    install -m 755 "$SRC_BIN" "$BIN" || fail "cannot write $BIN"
    # The zip was downloaded for the plugin deploy; bin/ gets the same copy so
    # the bundle stays coherent with its own refreshed SHA256SUMS.txt —
    # otherwise a later offline run would deploy a stale plugin against the
    # new sums.
    install -m 644 "$SRC_ZIP" "$BUNDLE_DIR/figma-plugin.zip" || fail "cannot write $BUNDLE_DIR/figma-plugin.zip"
    if [ "$ALL_PLATFORMS" -eq 1 ]; then
      for other in $ALL_ASSETS; do
        [ "$other" = "$ASSET" ] && continue
        [ -f "$TMP/$other" ] || continue
        install -m 644 "$TMP/$other" "$BUNDLE_DIR/$other" || fail "cannot write $BUNDLE_DIR/$other"
      done
    fi
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

  mkdir -p "$PLUGIN_DIR" || fail "cannot create $PLUGIN_DIR"
  rm -rf "$PLUGIN_DIR/plugin"
  extract "$SRC_ZIP" "$PLUGIN_DIR" || fail "cannot extract plugin into $PLUGIN_DIR (need unzip, or PowerShell on Windows)"
  [ -f "$MANIFEST" ] || fail "plugin manifest missing after extract: $MANIFEST"

  # macOS quarantines downloaded binaries; strip it so Gatekeeper does not
  # block the unsigned binary when the MCP client spawns it.
  if [ "$OS" = "darwin" ]; then
    xattr -d com.apple.quarantine "$BIN" 2>/dev/null
    xattr -cr "$PLUGIN_DIR/plugin" 2>/dev/null
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
