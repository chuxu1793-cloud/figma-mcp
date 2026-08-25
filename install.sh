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

mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Download binary
echo "→ Downloading $BINARY..."
curl -fsSL -o figma-mcp "$BASE_URL/$BINARY"
chmod +x figma-mcp

# Download plugin
echo "→ Downloading plugin..."
curl -fsSL -o figma-plugin.zip "$BASE_URL/figma-plugin.zip"
unzip -qo figma-plugin.zip
rm -f figma-plugin.zip

echo ""
echo "✓ Installation complete: $INSTALL_DIR"
echo ""
echo "MCP config:"
echo "  command: $INSTALL_DIR/figma-mcp"
echo ""
echo "Figma plugin manifest:"
echo "  $INSTALL_DIR/plugin/manifest.json"
