#!/bin/bash
# Idempotent installer for figma-mcp (binary + Figma plugin).
# Outputs machine-readable KEY: value lines only.
set -uo pipefail

REPO="chuxu1793-cloud/figma-mcp"
DIR="$HOME/figma"
TAG="latest"
FORCE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --dir)     DIR="$2"; shift 2 ;;
    --version) TAG="$2"; shift 2 ;;
    --force)   FORCE=1; shift ;;
    *) echo "STATUS: error"; echo "REASON: unknown argument $1"; exit 2 ;;
  esac
done

fail() { echo "STATUS: error"; echo "REASON: $1"; exit 1; }

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$OS-$ARCH" in
  darwin-arm64)  ASSET="figma-mcp-darwin-arm64" ;;
  darwin-x86_64) ASSET="figma-mcp-darwin-amd64" ;;
  linux-x86_64)  ASSET="figma-mcp-linux-amd64" ;;
  *) fail "unsupported platform $OS-$ARCH; prebuilt targets are darwin-arm64, darwin-amd64, linux-amd64, windows-amd64" ;;
esac

if [ "$TAG" = "latest" ]; then
  BASE="https://github.com/$REPO/releases/latest/download"
else
  BASE="https://github.com/$REPO/releases/download/$TAG"
fi

command -v curl >/dev/null || fail "curl not found"
command -v unzip >/dev/null || fail "unzip not found"
if command -v shasum >/dev/null; then SHA="shasum -a 256"
elif command -v sha256sum >/dev/null; then SHA="sha256sum"
else SHA=""; fi

BIN="$DIR/figma-mcp"
MANIFEST="$DIR/plugin/manifest.json"
TMP=$(mktemp -d) || fail "cannot create temp dir"
trap 'rm -rf "$TMP"' EXIT

# Expected checksum of the release asset, used both for verification and to
# decide whether the local binary is already up to date.
WANT=""
if [ -n "$SHA" ] && curl -fsSL --max-time 30 -o "$TMP/SHA256SUMS.txt" "$BASE/SHA256SUMS.txt"; then
  WANT=$(awk -v a="$ASSET" '$2 == a || $2 == "*"a {print $1}' "$TMP/SHA256SUMS.txt" | head -1)
fi

HAVE=""
if [ -n "$SHA" ] && [ -f "$BIN" ]; then
  HAVE=$($SHA "$BIN" | awk '{print $1}')
fi

mkdir -p "$DIR" || fail "cannot create $DIR"

if [ "$FORCE" -eq 0 ] && [ -n "$WANT" ] && [ "$HAVE" = "$WANT" ] && [ -f "$MANIFEST" ]; then
  echo "STATUS: already-current"
else
  curl -fsSL --max-time 300 -o "$TMP/$ASSET" "$BASE/$ASSET" || fail "download failed: $BASE/$ASSET"
  if [ -n "$WANT" ]; then
    GOT=$($SHA "$TMP/$ASSET" | awk '{print $1}')
    [ "$GOT" = "$WANT" ] || fail "checksum mismatch for $ASSET (expected $WANT, got $GOT)"
    CHECKSUM_STATE="verified"
  else
    CHECKSUM_STATE="skipped (no SHA256SUMS.txt or hashing tool)"
  fi

  curl -fsSL --max-time 300 -o "$TMP/figma-plugin.zip" "$BASE/figma-plugin.zip" || fail "download failed: $BASE/figma-plugin.zip"

  install -m 755 "$TMP/$ASSET" "$BIN" || fail "cannot write $BIN"
  rm -rf "$DIR/plugin"
  unzip -qo "$TMP/figma-plugin.zip" -d "$DIR" || fail "cannot unzip plugin into $DIR"

  # macOS quarantines curl-downloaded binaries; strip it so Gatekeeper does not
  # block the unsigned binary when the MCP client spawns it.
  if [ "$OS" = "darwin" ]; then
    xattr -d com.apple.quarantine "$BIN" 2>/dev/null
    xattr -cr "$DIR/plugin" 2>/dev/null
  fi

  echo "STATUS: installed"
  echo "CHECKSUM: $CHECKSUM_STATE"
fi

[ -x "$BIN" ] || fail "binary missing or not executable: $BIN"
[ -f "$MANIFEST" ] || fail "plugin manifest missing: $MANIFEST"

echo "BINARY: $BIN"
echo "MANIFEST: $MANIFEST"
echo "PLATFORM: $ASSET"
echo "SOURCE: $BASE"
