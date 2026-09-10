#!/bin/bash
# Finds figma-mcp server processes and reports which are stale; optionally kills them.
#
# Usage: cleanup.sh [--port 1994] [--apply] [--all]
#   (default)  dry run — report only, change nothing
#   --apply    kill the processes classified as stale
#   --all      treat every figma-mcp process as a target, including current ones
#              (only ever kill current ones when the user asked to stop everything;
#               a live MCP client session loses its server until restarted)
#
# Why this exists: the server only exits on stdin EOF or SIGINT. There is no idle
# timeout, so a process outlives an MCP client that was force-quit, and it keeps
# owning the port and the plugin WebSocket while running an outdated binary image.
#
# Stale means any of:
#   orphan          parent is gone (reparented to init)
#   image-deleted   the executable it runs was deleted/moved (e.g. to Trash)
#   image-outdated  running a different inode than the installed binary
#   image-foreign   running a binary outside this skill's bin/ (e.g. an old
#                   copied install under ~/figma)
#
# All stale processes are killed together on purpose: a follower takes over the
# port within 3-5s, so killing only the leader just promotes another stale one.
set -uo pipefail

PORT=1994
APPLY=0
ALL=0

while [ $# -gt 0 ]; do
  case "$1" in
    --port)  PORT="$2"; shift 2 ;;
    --apply) APPLY=1; shift ;;
    --all)   ALL=1; shift ;;
    *) echo "STATUS: error"; echo "REASON: unknown argument $1"; exit 2 ;;
  esac
done

SELF_DIR=$(cd "$(dirname "$0")" && pwd)
SKILL_DIR=$(cd "$SELF_DIR/.." && pwd)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case "$OS" in mingw*|msys*|cygwin*) OS="windows" ;; esac
ARCH=$(uname -m)
case "$OS-$ARCH" in
  darwin-arm64)   ASSET="figma-mcp-darwin-arm64" ;;
  darwin-x86_64)  ASSET="figma-mcp-darwin-amd64" ;;
  linux-x86_64)   ASSET="figma-mcp-linux-amd64" ;;
  windows-x86_64) ASSET="figma-mcp-windows-amd64.exe" ;;
  *)              ASSET="figma-mcp" ;;
esac
BIN="$SKILL_DIR/bin/$ASSET"
echo "PLATFORM: $OS"
echo "MODE: $([ "$APPLY" -eq 1 ] && echo apply || echo dry-run)"

# --- inode of the currently installed binary, for outdated-image detection ----
BIN_INODE=""
if [ -f "$BIN" ]; then
  case "$OS" in
    darwin) BIN_INODE=$(stat -f %i "$BIN" 2>/dev/null) ;;
    linux)  BIN_INODE=$(stat -c %i "$BIN" 2>/dev/null) ;;
  esac
  echo "INSTALLED: $BIN${BIN_INODE:+ (inode $BIN_INODE)}"
else
  echo "INSTALLED: missing ($BIN)"
fi

# --- which pid owns the port --------------------------------------------------
leader_pid() {
  case "$OS" in
    windows)
      netstat.exe -ano 2>/dev/null | awk -v p=":$PORT" \
        '$0 ~ /LISTENING/ && $2 ~ p"$" {print $NF; exit}'
      ;;
    *)
      if command -v lsof >/dev/null; then
        lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -1
      elif command -v ss >/dev/null; then
        ss -lptnH "sport = :$PORT" 2>/dev/null | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2
      elif command -v fuser >/dev/null; then
        fuser -n tcp "$PORT" 2>/dev/null | tr -s ' ' '\n' | grep -E '^[0-9]+$' | head -1
      fi
      ;;
  esac
}

# --- executable image a running pid actually uses -----------------------------
# Prints "path<TAB>inode"; either field may be empty when unavailable.
image_of() {
  local pid="$1"
  case "$OS" in
    linux)
      local exe
      exe=$(readlink "/proc/$pid/exe" 2>/dev/null)
      printf '%s\t%s\n' "$exe" "$(stat -c %i "/proc/$pid/exe" 2>/dev/null)"
      ;;
    darwin)
      command -v lsof >/dev/null || { printf '\t\n'; return; }
      # COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME — first txt entry is
      # the main executable image, even after the file was moved or deleted.
      lsof -p "$pid" 2>/dev/null | awk '$4 == "txt" {print $NF "\t" $8; exit}'
      ;;
    *) printf '\t\n' ;;
  esac
}

# --- every figma-mcp server process ------------------------------------------
# Matched on the executable token, not a substring of the whole command line,
# so this script and doctor.sh (whose own paths contain "figma-mcp") never
# match. The bundled asset names are enumerated exactly so unrelated servers
# such as figma-mcp-go still never match; plain "figma-mcp" catches processes
# from pre-in-place copied installs (e.g. ~/figma/figma-mcp).
list_procs() {
  case "$OS" in
    windows)
      for img in figma-mcp.exe figma-mcp-windows-amd64.exe; do
        tasklist.exe /FO CSV /NH /FI "IMAGENAME eq $img" 2>/dev/null |
          awk -F'","' '{gsub(/"/,"",$2); if ($2 ~ /^[0-9]+$/) print $2 "\t?\t?"}'
      done
      ;;
    *)
      ps -eo pid=,ppid=,etime=,args= 2>/dev/null | while IFS= read -r line; do
        set -- $line
        [ $# -ge 4 ] || continue
        pid="$1"; ppid="$2"; etime="$3"; shift 3
        first="$1"
        case "${first##*/}" in
          figma-mcp|figma-mcp.exe|figma-mcp-darwin-arm64|figma-mcp-darwin-amd64|figma-mcp-linux-amd64) printf '%s\t%s\t%s\n' "$pid" "$ppid" "$etime" ;;
        esac
      done
      ;;
  esac
}

LEADER=$(leader_pid)
if [ -n "$LEADER" ]; then
  VERSION=$(curl -fsS --max-time 3 "http://127.0.0.1:$PORT/ping" 2>/dev/null)
  echo "LEADER: pid=$LEADER${VERSION:+ ping=$VERSION}"
else
  echo "LEADER: nothing listening on 127.0.0.1:$PORT"
fi

TARGETS=""
COUNT=0
STALE_COUNT=0

while IFS=$'\t' read -r pid ppid etime; do
  [ -n "${pid:-}" ] || continue
  COUNT=$((COUNT + 1))

  IFS=$'\t' read -r img inode <<EOF
$(image_of "$pid")
EOF

  reasons=""
  case "$ppid" in
    1) reasons="orphan" ;;
  esac
  case "$img" in
    *" (deleted)"|*/.Trash/*|*/.Trash-*/*) reasons="${reasons:+$reasons,}image-deleted" ;;
  esac
  if [ -n "$img" ] && [ "$img" != "$BIN" ] && [ "${img% (deleted)}" != "$BIN" ]; then
    case "$reasons" in
      *image-deleted*) : ;;
      *) reasons="${reasons:+$reasons,}image-foreign" ;;
    esac
  elif [ -n "$inode" ] && [ -n "$BIN_INODE" ] && [ "$inode" != "$BIN_INODE" ]; then
    reasons="${reasons:+$reasons,}image-outdated"
  fi

  role="follower"
  [ "$pid" = "$LEADER" ] && role="leader"
  state="current"
  if [ -n "$reasons" ]; then state="stale ($reasons)"; STALE_COUNT=$((STALE_COUNT + 1)); fi

  echo "PROC: pid=$pid role=$role age=$etime ppid=$ppid state=$state image=${img:-unknown}"

  if [ -n "$reasons" ] || [ "$ALL" -eq 1 ]; then
    TARGETS="${TARGETS:+$TARGETS }$pid"
  fi
done <<EOF
$(list_procs)
EOF

echo "FOUND: $COUNT process(es), $STALE_COUNT stale"

if [ -z "$TARGETS" ]; then
  echo "STATUS: nothing to clean"
  [ "$COUNT" -gt 0 ] && echo "NOTE: remaining process(es) run the installed binary and belong to live MCP client sessions"
  exit 0
fi

echo "TARGETS: $TARGETS"

if [ "$APPLY" -eq 0 ]; then
  echo "STATUS: dry-run"
  echo "NEXT: re-run with --apply to kill the target process(es)"
  exit 0
fi

# SIGINT is the handled path (stops election, closes the WebSocket); SIGTERM and
# SIGKILL are unhandled fallbacks. Signal every target before waiting, so no
# surviving stale follower can win the port in between.
kill_all() {
  local sig="$1" pid left=""
  for pid in $TARGETS; do
    if kill -0 "$pid" 2>/dev/null; then
      case "$OS" in
        windows) taskkill.exe /PID "$pid" /F >/dev/null 2>&1 ;;
        *) kill -"$sig" "$pid" 2>/dev/null ;;
      esac
      echo "KILL: pid=$pid signal=$sig"
      left="${left:+$left }$pid"
    fi
  done
  [ -n "$left" ]
}

for sig in INT TERM KILL; do
  kill_all "$sig" || break
  sleep 2
  alive=""
  for pid in $TARGETS; do
    kill -0 "$pid" 2>/dev/null && alive="${alive:+$alive }$pid"
  done
  [ -n "$alive" ] || break
  TARGETS="$alive"
done

SURVIVORS=""
for pid in $TARGETS; do
  kill -0 "$pid" 2>/dev/null && SURVIVORS="${SURVIVORS:+$SURVIVORS }$pid"
done

if [ -n "$SURVIVORS" ]; then
  echo "STATUS: error"
  echo "REASON: process(es) still alive after SIGKILL: $SURVIVORS"
  exit 1
fi

# A non-stale process may legitimately take the freed port within 3-5s.
sleep 4
NEW=$(leader_pid)
if [ -n "$NEW" ]; then
  IFS=$'\t' read -r nimg _ <<EOF
$(image_of "$NEW")
EOF
  echo "PORT: now held by pid=$NEW image=${nimg:-unknown}"
else
  echo "PORT: free (127.0.0.1:$PORT)"
fi

echo "STATUS: cleaned"
echo "NEXT: reopen the Figma MCP plugin window if the bridge was in use — it reconnects on its own within ~1.5s"
echo "NEXT: run this skill's scripts/doctor.sh --test to confirm the bridge"
