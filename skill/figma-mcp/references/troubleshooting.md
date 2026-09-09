# figma-mcp troubleshooting

Contents: architecture detail · stale processes · symptom table · plugin import steps · Windows setup · env vars · manual config shapes

## Architecture detail beyond SKILL.md

- Multiple instances elect a **leader** by binding the port. Only the leader holds the WebSocket to the Figma plugin; followers proxy tool calls to `POST /rpc`. If the leader dies, a follower takes over within ~3–5s.
- `--ip` changes the bind address. There is no authentication, so keep it on loopback unless the user explicitly wants remote access.
- The plugin connects to `ws://<host>:<port>/ws`. There is no Connect button; the gear icon only changes host/port.
- Host/port entered in the plugin persist via `figma.clientStorage`, not localStorage — they survive restarts and are per Figma user.

## Stale processes on the port

### Why they accumulate

The process has exactly two exit paths: stdin EOF (the MCP client closes the pipe) and SIGINT. There is no idle timeout, no heartbeat-based self-eviction, and the election monitor only ever *promotes* — it never asks an existing leader to step down. Consequences:

- A client that is SIGKILLed, or a terminal window force-closed, can leave the child with an open stdin. A client session simply left open for days keeps its server alive just as long.
- `install.sh` writes a new file; a running process keeps executing the image it started with (macOS shows the old image under `~/.Trash/...` after the file was replaced). The stale leader keeps the port, a freshly spawned server becomes a follower, and **every tool call is proxied into the old code** — new tools appear missing, fixed bugs come back.
- `GET /ping` reports that old process's version, so a version older than the release you just installed is the cheapest skew signal.
- Killing only the leader promotes a stale follower within 3–5s. The whole stale set has to go at once — which is why `cleanup.sh` signals all targets before waiting.

### Detect

`doctor.sh` surfaces this automatically (`PROC:` / `FOUND:` lines, plus a `NEXT:` pointing at `cleanup.sh`). Manual equivalents:

| | macOS | Linux | Windows |
|---|---|---|---|
| Who owns the port | `lsof -nP -iTCP:1994 -sTCP:LISTEN` | `ss -lptn 'sport = :1994'` | `netstat -ano \| findstr :1994` |
| Parent + age | `ps -o pid,ppid,lstart,etime,args -p <pid>` | same | `tasklist /FI "PID eq <pid>"` |
| Which image it runs | `lsof -p <pid> \| awk '$4=="txt"{print $NF; exit}'` | `readlink /proc/<pid>/exe` | not available |
| Version being served | `curl -s http://127.0.0.1:1994/ping` | same | same |

Stale indicators: `ppid` is 1 (orphan), image path under `.Trash` or suffixed ` (deleted)`, image inode different from the installed binary, image outside the install dir, or a `/ping` version behind the installed release.

### Clean up

```bash
bash SKILL_DIR/scripts/cleanup.sh --dir ~/figma            # report only
bash SKILL_DIR/scripts/cleanup.sh --dir ~/figma --apply    # SIGINT -> SIGTERM -> SIGKILL
```

SIGINT is the handled path (stops the election monitor and closes the WebSocket); SIGTERM/SIGKILL are unhandled fallbacks that terminate immediately — nothing is persisted, so there is no state to corrupt. The Figma plugin reconnects by itself within ~1.5s. `--all` also stops healthy processes; the client sessions owning them must be restarted.

Do not use `pkill -f figma-mcp` / `killall`: the pattern also matches this skill's scripts and unrelated servers such as `@scope/figma-mcp-go`.

### Prevent

- Quit MCP clients normally so stdin reaches EOF; avoid force-quitting them or force-closing their terminal window.
- Do not keep many old client sessions open — each one holds its own server process.
- After every upgrade, act on `install.sh`'s `STALE:` line instead of assuming the new binary is in use.
- No env var fixes this. `FIGMA_MCP_ELECTION_JITTER_MIN`/`_MAX` only change how fast a takeover happens after a leader dies; they cannot evict a live one.

## Symptom table

| Symptom | Cause | Fix |
|---|---|---|
| Tool returns `plugin not connected` | Plugin not open in Figma, or open but pointed at another port | Open the plugin in the target Figma file; check gear icon host/port matches the server's `--port` |
| Tool returns `request timed out` | Plugin busy, huge subtree, or Figma tab backgrounded | Retry with a narrower `nodeId`/`depth`; raise `FIGMA_MCP_TIMEOUT` (see below) |
| Tools absent from the client after registering | Client caches MCP config at startup | Restart the client session; verify the entry with `doctor.sh` |
| macOS: "cannot be opened because the developer cannot be verified" | Binary is unsigned and quarantined | `xattr -d com.apple.quarantine <binary>` (install.sh does this automatically) |
| `zsh: bad CPU type in executable` | Intel binary on Apple Silicon or vice versa | Reinstall with `install.sh --force`; it selects by `uname -m` |
| Leader never starts, port error at launch | Another process owns the port | Start with `--port N` **and** set the same port in the plugin gear dialog |
| Two clients, only one sees Figma data | Expected: follower proxies through the leader | No action; if the leader was killed, wait ~5s for takeover |
| `/ping` version older than the release just installed; new tools missing after upgrade | A pre-upgrade process still owns the port and serves the old image | `cleanup.sh --apply` (see above), then re-verify with `doctor.sh --test` |
| `doctor.sh --test` never starts a temporary server | Port already held, often by an orphaned process | Identify with `cleanup.sh`, clear it, re-run `--test` |
| Bridge dies for every client at once, long after the Figma side looked fine | Stale leader was killed by the OS or lost its plugin socket; followers proxy into it | `cleanup.sh --apply`, reopen the plugin window |
| Plugin missing from Figma's menu | Manifest not imported, or imported from a deleted path | Re-import `<dir>/plugin/manifest.json`; keep the directory in place |
| Import option greyed out / absent | Using Figma in a browser | Development plugins require the Figma **Desktop** app |
| `--version` fails with "unexpected argument" | Flag does not exist | Read the version from `GET /ping`, or compare the binary's sha256 against `SHA256SUMS.txt` |
| Linux arm64 has no asset | Only darwin-arm64, darwin-amd64, linux-amd64, windows-amd64 are built | Build from source, or run the amd64 build under emulation |

## Figma plugin import (GUI only — cannot be scripted)

Prerequisites: Figma **Desktop** app only (a browser tab will not work), and any file open — the Plugins menu appears only inside an open file.

1. Open the **Figma Desktop** app (`open -a Figma` on macOS).
2. Menu: **Plugins > Development > Import plugin from manifest…**
3. Select `<install-dir>/plugin/manifest.json` (default `~/figma/plugin/manifest.json`).
4. In any open file, run **Plugins > Development > Figma MCP**.
5. The plugin window must stay open — closing it drops the WebSocket. It reconnects on its own when reopened.

One-time only: after step 3 the plugin stays in the Development menu.

When guiding a user live, follow SKILL.md step 5: work step by step with announce → ask — state the operation in a text message first, then pop the result dialog — and verify with `doctor.sh --test`. Never fire a dialog the user cannot understand on its own; the question text must restate the operation and its expected outcome.

## Windows without Git Bash or WSL

`install.sh` and `doctor.sh` need a POSIX shell. With Git Bash or WSL they work as-is (the `.exe` asset is selected automatically). Without one, do this in PowerShell:

```powershell
$dir = "$env:USERPROFILE\figma"; New-Item -ItemType Directory -Force $dir | Out-Null
$base = "https://github.com/chuxu1793-cloud/figma-mcp/releases/latest/download"
Invoke-WebRequest "$base/figma-mcp-windows-amd64.exe" -OutFile "$dir\figma-mcp.exe"
Invoke-WebRequest "$base/figma-plugin.zip" -OutFile "$dir\figma-plugin.zip"
Expand-Archive -Force "$dir\figma-plugin.zip" $dir; Remove-Item "$dir\figma-plugin.zip"
Invoke-WebRequest "$base/SHA256SUMS.txt" -OutFile "$dir\SHA256SUMS.txt"
(Get-FileHash "$dir\figma-mcp.exe" -Algorithm SHA256).Hash.ToLower()   # compare with SHA256SUMS.txt
```

Then register manually with a double-escaped path — `"command": "C:\\Users\\you\\figma\\figma-mcp.exe"` — using the shapes below, and import `plugin\manifest.json` in Figma Desktop.

Verification without the scripts: `curl http://127.0.0.1:1994/ping` while an MCP client session is running.

## Linux

Binary and registration work normally (`figma-mcp-linux-amd64`; no arm64 asset). Figma ships no official Linux desktop app, and development plugins cannot be imported in a browser tab — so the plugin bridge cannot be established with official software. Options: run the client/plugin on a macOS or Windows machine and point the plugin at that host's port, or use an unofficial Figma Linux build (untested here).

## Environment variables

| Variable | Default | Effect |
|---|---|---|
| `FIGMA_MCP_TIMEOUT` | 30 | Bridge timeout in seconds for all tools except `get_design_context` |
| `FIGMA_MCP_TIMEOUT_DESIGN_CONTEXT` | 60 | Bridge timeout for `get_design_context` |
| `FIGMA_MCP_ELECTION_JITTER_MIN` / `_MAX` | 3000 / 5000 | Leader health-check interval in ms |
| `RUST_LOG` | `figma_mcp=info` | Log filter; logs go to stderr |

Pass them via the MCP client's `env` block, e.g. `"env": { "FIGMA_MCP_TIMEOUT": "60" }`.

## Manual config shapes

Only needed when `register_client.cjs` refuses the file (e.g. JSON with comments).

`mcpServers` shape — Codely (`~/.codely-cli/settings.json`), Claude Desktop, Claude Code (`.mcp.json`), Cursor (`~/.cursor/mcp.json`):

```json
{ "mcpServers": { "figma": { "command": "/Users/you/figma/figma-mcp" } } }
```

`servers` shape — VS Code / Copilot (`.vscode/mcp.json`):

```json
{ "servers": { "figma": { "type": "stdio", "command": "/Users/you/figma/figma-mcp" } } }
```

Claude Code CLI alternative: `claude mcp add -s project figma -- /Users/you/figma/figma-mcp`

Non-default port: add `"args": ["--port", "1995"]` and set the same port in the plugin.
