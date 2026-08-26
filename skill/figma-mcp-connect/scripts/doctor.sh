#!/bin/bash
# Reports figma-mcp setup state and what is still missing.
# Usage: doctor.sh [--dir ~/figma] [--port 1994] [--test]
#   --test  temporarily starts the binary itself to probe the plugin bridge
#           (only when nothing is listening on the port)
set -uo pipefail

DIR="$HOME/figma"
PORT=1994
TEST=0

while [ $# -gt 0 ]; do
  case "$1" in
    --dir)  DIR="$2"; shift 2 ;;
    --port) PORT="$2"; shift 2 ;;
    --test) TEST=1; shift ;;
    *) echo "REASON: unknown argument $1"; exit 2 ;;
  esac
done

BIN="$DIR/figma-mcp"
MANIFEST="$DIR/plugin/manifest.json"
NEXT=()

# --- binary + plugin ---------------------------------------------------------
if [ -x "$BIN" ]; then
  echo "BINARY: $BIN"
else
  echo "BINARY: missing ($BIN)"
  NEXT+=("run scripts/install.sh")
fi

if [ -f "$MANIFEST" ]; then
  echo "MANIFEST: $MANIFEST"
else
  echo "MANIFEST: missing ($MANIFEST)"
  NEXT+=("run scripts/install.sh")
fi

if [ "$(uname -s)" = "Darwin" ] && [ -f "$BIN" ]; then
  if xattr -p com.apple.quarantine "$BIN" >/dev/null 2>&1; then
    echo "QUARANTINE: present (Gatekeeper will block launch)"
    NEXT+=("xattr -d com.apple.quarantine \"$BIN\"")
  else
    echo "QUARANTINE: clear"
  fi
fi

# --- MCP client configs referencing the binary -------------------------------
FOUND=()
for f in "$HOME/.codely-cli/settings.json" \
         "$HOME/Library/Application Support/Claude/claude_desktop_config.json" \
         "$HOME/.config/Claude/claude_desktop_config.json" \
         "$HOME/.cursor/mcp.json" \
         "$PWD/.mcp.json" "$PWD/.cursor/mcp.json" "$PWD/.vscode/mcp.json"; do
  # Match a command ending in figma-mcp only, so unrelated servers such as
  # "@scope/figma-mcp-go" are not miscounted as this binary.
  [ -f "$f" ] && grep -Eq '"[^"]*figma-mcp(\.exe)?"' "$f" 2>/dev/null && FOUND+=("$f")
done
if [ ${#FOUND[@]} -gt 0 ]; then
  echo "REGISTERED_IN: ${FOUND[*]}"
else
  echo "REGISTERED_IN: none found"
  NEXT+=("run scripts/register_client.cjs --client <id> --binary \"$BIN\"")
fi

# --- Figma desktop app -------------------------------------------------------
if [ "$(uname -s)" = "Darwin" ]; then
  if pgrep -qx Figma 2>/dev/null; then echo "FIGMA_APP: running"; else
    echo "FIGMA_APP: not running"
    NEXT+=("open -a Figma")
  fi
fi

# --- leader + bridge ---------------------------------------------------------
probe() {
  local ping_json bridge_json
  ping_json=$(curl -fsS --max-time 3 "http://127.0.0.1:$PORT/ping" 2>/dev/null) || return 1
  echo "LEADER: up ($ping_json)"
  bridge_json=$(curl -fsS --max-time 8 -H 'Content-Type: application/json' \
    -d '{"tool":"get_metadata"}' "http://127.0.0.1:$PORT/rpc" 2>/dev/null)
  case "$bridge_json" in
    *'"error":"plugin not connected"'*)
      echo "BRIDGE: plugin not connected"
      NEXT+=("open the Figma MCP plugin inside a Figma file (Plugins - Development - Figma MCP); it auto-connects and retries every 1.5s")
      ;;
    *'"data"'*) echo "BRIDGE: connected ${bridge_json:0:200}" ;;
    *)          echo "BRIDGE: unexpected response ${bridge_json:0:200}" ;;
  esac
  return 0
}

if probe; then
  :
elif [ "$TEST" -eq 1 ] && [ -x "$BIN" ]; then
  # Hold stdin open through a FIFO: the MCP stdio transport exits on EOF.
  TMP=$(mktemp -d)
  mkfifo "$TMP/in"
  exec 9<>"$TMP/in"
  "$BIN" --port "$PORT" <&9 >/dev/null 2>"$TMP/err" &
  PID=$!
  sleep 2
  echo "PROBE: started temporary server (pid $PID)"
  probe || echo "LEADER: failed to start — $(tail -2 "$TMP/err" | tr '\n' ' ')"
  kill "$PID" 2>/dev/null
  wait "$PID" 2>/dev/null
  exec 9>&-
  rm -rf "$TMP"
  echo "PROBE: temporary server stopped"
else
  echo "LEADER: nothing listening on 127.0.0.1:$PORT"
  echo "NOTE: normal when no MCP client session is running — the client spawns the server on demand"
  NEXT+=("re-run with --test to probe the bridge without an MCP client")
fi

for n in "${NEXT[@]}"; do echo "NEXT: $n"; done
[ ${#NEXT[@]} -eq 0 ] && echo "NEXT: none — setup looks complete"
exit 0
