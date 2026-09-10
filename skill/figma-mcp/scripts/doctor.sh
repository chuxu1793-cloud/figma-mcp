#!/bin/bash
# Reports figma-mcp setup state and what is still missing.
# Usage: doctor.sh [--port 1994] [--test]
#   --test  temporarily starts the binary itself to probe the plugin bridge
#           (only when nothing is listening on the port)
set -uo pipefail

PORT=1994
TEST=0

while [ $# -gt 0 ]; do
  case "$1" in
    --port) PORT="$2"; shift 2 ;;
    --test) TEST=1; shift ;;
    *) echo "REASON: unknown argument $1"; exit 2 ;;
  esac
done

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in mingw*|msys*|cygwin*) OS="windows" ;; esac
ARCH=$(uname -m)
SELF_DIR=$(cd "$(dirname "$0")" && pwd)
SKILL_DIR=$(cd "$SELF_DIR/.." && pwd)
case "$OS-$ARCH" in
  darwin-arm64)   ASSET="figma-mcp-darwin-arm64" ;;
  darwin-x86_64)  ASSET="figma-mcp-darwin-amd64" ;;
  linux-x86_64)   ASSET="figma-mcp-linux-amd64" ;;
  windows-x86_64) ASSET="figma-mcp-windows-amd64.exe" ;;
  *)              ASSET="figma-mcp" ;;
esac
BIN="$SKILL_DIR/bin/$ASSET"
MANIFEST="$SKILL_DIR/plugin/manifest.json"
NEXT=()
echo "PLATFORM: $OS"

# --- binary + plugin ---------------------------------------------------------
NEED_INSTALL=0
if [ -x "$BIN" ]; then
  echo "BINARY: $BIN"
else
  echo "BINARY: missing or not executable ($BIN)"
  NEED_INSTALL=1
fi

if [ -f "$MANIFEST" ]; then
  echo "MANIFEST: $MANIFEST"
else
  echo "MANIFEST: missing ($MANIFEST)"
  NEED_INSTALL=1
fi

[ "$NEED_INSTALL" -eq 1 ] && NEXT+=("run this skill's scripts/install.sh")

if [ "$OS" = "darwin" ] && [ -f "$BIN" ]; then
  if xattr -p com.apple.quarantine "$BIN" >/dev/null 2>&1; then
    echo "QUARANTINE: present (Gatekeeper will block launch)"
    NEXT+=("xattr -d com.apple.quarantine \"$BIN\"")
  else
    echo "QUARANTINE: clear"
  fi
fi

# --- MCP client configs referencing the binary -------------------------------
# A current registration points at this skill's bundled asset by name. A
# figma-mcp command that is not this asset (e.g. an old copy under ~/figma)
# is reported as outdated: it still runs, but it defeats in-place upgrades,
# so the config must be re-registered.
FOUND=()
OUTDATED=()
for f in "$HOME/.codely-cli/settings.json" \
         "$HOME/Library/Application Support/Claude/claude_desktop_config.json" \
         "$HOME/.config/Claude/claude_desktop_config.json" \
         "${APPDATA:-$HOME/AppData/Roaming}/Claude/claude_desktop_config.json" \
         "$HOME/.cursor/mcp.json" \
         "$PWD/.mcp.json" "$PWD/.cursor/mcp.json" "$PWD/.vscode/mcp.json"; do
  [ -f "$f" ] || continue
  if grep -Fq "$ASSET" "$f" 2>/dev/null; then
    FOUND+=("$f")
  elif grep -Eq '"[^"]*figma-mcp(\.exe)?"' "$f" 2>/dev/null; then
    OUTDATED+=("$f")
  fi
done
if [ ${#FOUND[@]} -gt 0 ]; then
  echo "REGISTERED_IN: ${FOUND[*]}"
fi
if [ ${#OUTDATED[@]} -gt 0 ]; then
  echo "REGISTERED_OLD: ${OUTDATED[*]} (figma-mcp command outside this skill folder)"
  NEXT+=("run this skill's scripts/register_client.cjs --client <id> --binary \"$BIN\" so the config points into the skill folder")
fi
if [ ${#FOUND[@]} -eq 0 ] && [ ${#OUTDATED[@]} -eq 0 ]; then
  echo "REGISTERED_IN: none found"
  NEXT+=("run this skill's scripts/register_client.cjs --client <id> --binary \"$BIN\"")
fi

# --- Figma desktop app -------------------------------------------------------
case "$OS" in
  darwin)
    if pgrep -qx Figma 2>/dev/null; then echo "FIGMA_APP: running"; else
      echo "FIGMA_APP: not running"
      NEXT+=("open -a Figma")
    fi
    ;;
  windows)
    if tasklist.exe 2>/dev/null | grep -qi '^figma\.exe'; then echo "FIGMA_APP: running"; else
      echo "FIGMA_APP: not running"
      NEXT+=("launch Figma Desktop from the Start menu")
    fi
    ;;
  *)
    echo "FIGMA_APP: unknown (no official Figma Desktop app on this OS)"
    ;;
esac

# --- running server processes, stale ones first ------------------------------
# Detection lives in cleanup.sh (dry run changes nothing); only its per-process
# lines are surfaced here. Runs before --test so the temporary probe server
# started below is never mistaken for a leftover.
if [ -x "$SELF_DIR/cleanup.sh" ]; then
  SCAN=$(bash "$SELF_DIR/cleanup.sh" --port "$PORT" 2>/dev/null)
  echo "$SCAN" | grep -E '^(PROC|FOUND):' || true
  STALE_N=$(echo "$SCAN" | sed -n 's/^FOUND: [0-9]* process(es), \([0-9]*\) stale$/\1/p')
  if [ -n "${STALE_N:-}" ] && [ "$STALE_N" -gt 0 ]; then
    NEXT+=("run this skill's scripts/cleanup.sh --port $PORT --apply to drop $STALE_N stale process(es) holding the port with an outdated binary")
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
      NEXT+=("open the Figma MCP plugin inside a Figma file (Plugins > Development > Figma MCP); it auto-connects and retries every 1.5s")
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

# Guarded: bash 3.2 (macOS system bash) treats "${arr[@]}" on an empty array as
# an unbound variable under `set -u`.
if [ ${#NEXT[@]} -gt 0 ]; then
  for n in "${NEXT[@]}"; do echo "NEXT: $n"; done
else
  echo "NEXT: none — setup looks complete"
fi
exit 0
