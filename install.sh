#!/bin/bash
set -e

INSTALL_DIR="${1:-$HOME/figma}"
REPO="chuxu1793-cloud/figma-mcp"
VERSION="v0.1.0"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS-$ARCH" in
  darwin-arm64)  BINARY="figma-mcp-darwin-arm64" ;;
  darwin-x86_64) BINARY="figma-mcp-darwin-amd64" ;;
  linux-x86_64)  BINARY="figma-mcp-linux-amd64" ;;
  linux-aarch64) BINARY="figma-mcp-linux-amd64" ;;
  *) echo "Unsupported platform: $OS-$ARCH"; exit 1 ;;
esac

mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Download binary
echo "→ Downloading $BINARY..."
curl -sL -o figma-mcp "https://github.com/$REPO/releases/download/$VERSION/$BINARY"
chmod +x figma-mcp

# Download plugin
echo "→ Downloading plugin..."
curl -sL -o figma-plugin.zip "https://github.com/$REPO/releases/download/$VERSION/figma-plugin.zip"
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
