#!/bin/bash
set -euo pipefail

INSTALL_DIR="${1:-$HOME/figma}"
REPO="chuxu1793-cloud/figma-mcp"
# 默认安装最新版本；可用 VERSION=v0.1.0 覆盖
VERSION="${VERSION:-latest}"

if [ "$VERSION" = "latest" ]; then
  BASE_URL="https://github.com/$REPO/releases/latest/download"
else
  BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
fi

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS-$ARCH" in
  darwin-arm64)  BINARY="figma-mcp-darwin-arm64" ;;
  darwin-x86_64) BINARY="figma-mcp-darwin-amd64" ;;
  linux-x86_64)  BINARY="figma-mcp-linux-amd64" ;;
  *)
    echo "Unsupported platform: $OS-$ARCH" >&2
    echo "Prebuilt binaries: darwin-arm64, darwin-amd64, linux-amd64, windows-amd64" >&2
    exit 1
    ;;
esac

command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v unzip >/dev/null || { echo "unzip is required" >&2; exit 1; }

if command -v shasum >/dev/null; then SHA="shasum -a 256"
elif command -v sha256sum >/dev/null; then SHA="sha256sum"
else SHA=""; fi

mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Expected checksums, when available
SUMS=""
if [ -n "$SHA" ] && curl -fsSL --max-time 30 -o SHA256SUMS.txt "$BASE_URL/SHA256SUMS.txt"; then
  SUMS="SHA256SUMS.txt"
fi

verify() { # verify <file> <asset-name>
  [ -n "$SUMS" ] || return 0
  want=$(awk -v a="$2" '$2 == a || $2 == "*"a {print $1}' "$SUMS" | head -1)
  [ -n "$want" ] || return 0
  got=$($SHA "$1" | awk '{print $1}')
  if [ "$got" != "$want" ]; then
    echo "✗ checksum mismatch for $2 (expected $want, got $got)" >&2
    exit 1
  fi
  echo "  checksum ok"
}

# Download binary
echo "→ Downloading $BINARY..."
curl -fsSL -o figma-mcp "$BASE_URL/$BINARY"
verify figma-mcp "$BINARY"
chmod +x figma-mcp

# Download plugin
echo "→ Downloading plugin..."
curl -fsSL -o figma-plugin.zip "$BASE_URL/figma-plugin.zip"
verify figma-plugin.zip figma-plugin.zip
unzip -qo figma-plugin.zip
rm -f figma-plugin.zip "$SUMS"

echo ""
echo "✓ Installation complete: $INSTALL_DIR"
echo ""
echo "MCP config:"
echo "  command: $INSTALL_DIR/figma-mcp"
echo ""
echo "Figma plugin manifest:"
echo "  $INSTALL_DIR/plugin/manifest.json"
