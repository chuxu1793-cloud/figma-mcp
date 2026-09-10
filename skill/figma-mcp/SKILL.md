---
name: figma-mcp
description: Get the figma-mcp server (Rust Figma MCP with plugin bridge, no Figma API token) installed, registered in an MCP client, connected to the Figma plugin, verified, and unblocked. Use when the user wants to install, update, configure, start, or connect figma-mcp in Codely / Claude / Cursor / VS Code, import or open the Figma plugin, or when Figma MCP tools fail with errors like "plugin not connected", "request timed out", tools missing after config, Gatekeeper blocking the binary, or port 1994 conflicts. Triggers on phrasings like "安装 figma-mcp", "配置 figma mcp", "启动 figma mcp", "figma mcp 连不上", "插件没连上", "install figma-mcp", "figma mcp not working".
---

# figma-mcp connect

Automate everything that can be automated. Probe the bridge with `doctor.sh --test` before prompting anything; when Figma GUI steps are unavoidable, guide step by step with **announce → ask** — state the operation first in a text message, then ask about its result with the `ask_user` dialog — and let the script verify at the end.

`SKILL_DIR` below = the absolute directory containing this SKILL.md (shown in the activation notice). Never guess it, and never retype script contents — call the scripts.

## Facts that drive the workflow

- Distribution: prebuilt binaries only — no source build, no npm package, no Figma API token. The skill bundles every target (darwin-arm64, darwin-amd64, linux-amd64, windows-amd64) plus `figma-plugin.zip`, `SHA256SUMS.txt`, and `VERSION` in `bin/`. **The server binary and the Figma plugin live in different places**: the binary runs in place from `SKILL_DIR/bin/` — never copied elsewhere, MCP configs point directly at it — while the plugin deploys OUT of the skill folder to a user-choosable visible directory (`--plugin-dir`, default `~/figma`, layout `<plugin-dir>/plugin/`), because it is the one path a human must navigate, in Figma's import file picker. `--version <tag>` downloads from GitHub releases of `chuxu1793-cloud/figma-mcp` into `bin/` in place — use it for releases newer than the bundled one.
- The binary is an MCP **stdio** server — the MCP client spawns it on demand. Never "start it as a service"; nothing listening between sessions is normal.
- After setup, two paths must stay in place: the skill folder (the registered MCP command points into `SKILL_DIR/bin/`) and the deployed plugin directory (the imported Figma plugin points at `<plugin-dir>/plugin/`). Moving or deleting either breaks the setup.
- Each process also serves `127.0.0.1:1994` (`GET /ping`, `POST /rpc`, `GET /ws`) and elects a leader by binding the port.
- A process exits only on stdin EOF or SIGINT — no idle timeout, no self-eviction. So a server outlives a force-quit client or a session left open for days, keeps the port and the plugin WebSocket, and keeps executing its **original** binary image after an upgrade. Every tool call then runs old code. Treat stale processes as a routine check, not an exotic failure.
- The Figma plugin auto-connects on open and retries every 1.5s. Importing/opening it is GUI-only and **cannot** be scripted.

## Platform matrix — check the OS before choosing commands

| | macOS | Windows | Linux |
|---|---|---|---|
| `install.sh` / `doctor.sh` | native | needs Git Bash or WSL (auto-detects MSYS/Cygwin, uses the `.exe` asset, falls back to PowerShell `Expand-Archive` when `unzip` is absent) | native |
| `register_client.cjs` | native | native (`node` on Windows works; Claude Desktop path resolves to `%APPDATA%\Claude`) | native |
| Launch Figma | `open -a Figma` | Start menu — no reliable CLI hook; do not invent one | **no official Figma Desktop app** → plugin bridge unavailable |
| Reveal plugin folder | `open <plugin-dir>/plugin` | `explorer.exe <plugin-dir>\plugin` (Git Bash) or `explorer <plugin-dir>\plugin` (cmd/PowerShell) | `xdg-open <plugin-dir>/plugin` |

Determine the OS from the environment context or `uname -s` before emitting any command. On Linux, install and register normally, but state the plugin-bridge limitation up front — see the Linux section in `references/troubleshooting.md` for the details and workarounds. If Windows has no Git Bash or WSL, follow the manual sequence in `references/troubleshooting.md` instead of inventing shell commands.

## Workflow

All scripts print `KEY: value` lines; `STATUS: error` plus `REASON:` means failure. Report those lines back rather than paraphrasing them.

### 1. Assess before acting

```bash
bash SKILL_DIR/scripts/doctor.sh [--plugin-dir <path>] [--port 1994]
```

Act only on the `NEXT:` lines it prints. Skip to step 5 if nothing else is missing. `PROC:` lines whose `state=` starts with `stale` mean step 3 is required — a stale process silently serves every tool call.

### 2. Prepare or update (server in place, plugin deployed out)

```bash
bash SKILL_DIR/scripts/install.sh [--plugin-dir <path>] [--version <tag>] [--force]
```

Idempotent and offline by default: verifies sha256 of the bundled binary and plugin zip against `SKILL_DIR/bin/SHA256SUMS.txt`, marks the binary executable in place, strips the macOS quarantine attribute, and deploys the plugin to `<plugin-dir>/plugin/` (replacing the previous copy). Reports `already-current` when checksum, executable bit, and the deployed manifest all check out. The server binary never leaves `SKILL_DIR/bin/`.

Ask the user for `--plugin-dir` before the first deploy — announce → ask, free-form path: state that the Figma plugin will be copied to `<path>/plugin/` and that Figma will import its manifest from there. If they have no preference, omit the flag and the default `~/figma` is used. The choice is not persisted anywhere: when `doctor.sh` already reports the deployed manifest, reuse that path on upgrades without re-asking; when it reports the manifest missing, ask again (if the user previously chose a custom path, they state it — doctor cannot remember it).

`--force` re-deploys (and downgrades). `--version <tag>` installs that GitHub release into `SKILL_DIR/bin/` in place, refreshing `bin/SHA256SUMS.txt` and `bin/VERSION` so later offline runs stay consistent. Without a bundled asset for the current platform it falls back to the GitHub release (online).

Omit `--version` (defaults to the bundled release) unless the user names a tag; `--version <tag>` or `--version latest` installs that release from GitHub into `bin/` in place. Do not invent tags or version numbers, and do not state a version unless a script or `GET /ping` reported it — the binary has no `--version` flag.

### 3. Clear stale processes (whenever `doctor.sh`/`install.sh` reports any)

```bash
bash SKILL_DIR/scripts/cleanup.sh [--port 1994]           # dry run: report only
bash SKILL_DIR/scripts/cleanup.sh [--port 1994] --apply   # kill the stale ones
```

Dry run first, always — read the `PROC:` lines aloud to the user before killing anything. Classifications: `orphan` (parent gone), `image-deleted` (executable removed or moved to Trash), `image-outdated` (different inode than the installed binary), `image-foreign` (binary outside `SKILL_DIR/bin/` — e.g. an old copy under `~/figma`). `state=current` processes belong to live client sessions and are left alone.

`--apply` signals every target before waiting (SIGINT → SIGTERM → SIGKILL). Do not hand-kill just the leader: a stale follower takes the port over within 3–5s, so the whole stale set must go at once. Add `--all` only when the user explicitly wants every figma-mcp process stopped, and say plainly that live client sessions will lose their server until restarted.

Killing is safe for the Figma side — the plugin reconnects on its own within ~1.5s.

After an upgrade this step is mandatory: `install.sh` replaces the file on disk but cannot restart a running server, so `install.sh` prints `STALE: n …` when old images are still live.

### 4. Register in the MCP client

```bash
node SKILL_DIR/scripts/register_client.cjs --client <id> --binary <BINARY: path from step 2> [--name figma]
```

Valid `--client` ids, exactly these: `codely`, `claude-desktop`, `claude-code` (project `.mcp.json`), `cursor`, `cursor-project`, `vscode` (project `.vscode/mcp.json`). Add `--config <path>` for a non-default location. Never pass an id outside this list; the script rejects unknown ids and prints the valid set.

Pick the id from context — when running inside Codely CLI and the user names no other tool, use `codely`. Ask only if the target is genuinely ambiguous. The script backs up the file, writes the right shape per client, and is idempotent. Take `--binary` from step 2's `BINARY:` line (e.g. `SKILL_DIR/bin/figma-mcp-darwin-arm64`): the config must point into the skill folder, never at a copied binary outside it.

If it reports a JSON parse error (comments in `.vscode/mcp.json` are common), edit the file manually using the shapes in `references/troubleshooting.md`.

Then tell the user to restart the client session — MCP config is read at startup.

### 5. Bring up the Figma side — probe first, then announce → ask steps

Probe before prompting: run `bash SKILL_DIR/scripts/doctor.sh --test` (step 6's command).

- `BRIDGE: connected` → the plugin is already imported and its window is open. Report success and stop — nothing to ask.
- Otherwise → automate the openable parts first, using the row for the current OS in the platform matrix above: launch Figma Desktop where a CLI hook exists, and reveal the folder holding `manifest.json`. Report what you opened.

Then guide step by step. The rule governing every round is **announce → ask**: first send a short text message stating the operation to perform (menu path, plus the expected visible outcome), and only then ask about the result with the `ask_user` dialog. Move on only after the user's explicit answer — if the reply is unclear, re-ask, never guess. Never fire a dialog the user has not been prepared for. Make every question self-contained: restate the operation and its expected outcome inside the question itself, so even a user who reads only the question never answers blind. Write menu paths as `Menu > Submenu > Item` (e.g. `Plugins > Development > Figma MCP`); space- or arrow-joined items blur into one line of words. Every `Ask:` in the sequence below follows this rule.

Sequence:

1. Announce: use the Figma **Desktop** app (a browser tab will not work; Figma exposes no automation hook for these steps), open any Figma file — the Plugins menu appears only inside an open file — and check whether **Plugins > Development** already lists *Figma MCP*. Ask: does the menu list Figma MCP? Yes → step 3; no / not sure → step 2.
2. Announce: run **Plugins > Development > Import plugin from manifest…** and select the deployed `<plugin-dir>/plugin/manifest.json` (step 2's `MANIFEST:` line — default `~/figma/plugin/manifest.json`) in the file picker. If that path is hidden or awkward to reach in the picker, say how: on macOS press `Cmd+Shift+G` and paste the full manifest path, or press `Cmd+Shift+.` to reveal hidden folders; on Windows paste the path into the file name box. Ask: did the import finish? If the user cannot find the menu, repeat the prerequisites from step 1 (file open, Desktop app) before re-asking.
3. Announce: in any open file, run **Plugins > Development > Figma MCP**; keep the plugin window open — closing it drops the bridge, and it reconnects on its own when reopened. Ask: is the plugin window showing?
4. Move straight to step 6: its bridge check is the real confirmation that step 3 worked. If it reports not connected, wait ~3s, check once more, then troubleshoot (plugin gear host/port, symptom table in `references/troubleshooting.md`).

Do not claim to have imported or opened the plugin — only the user's confirmations and step 6's `BRIDGE:` line count as evidence. If the user explicitly asks for all steps at once, give the import list in `references/troubleshooting.md` instead of walking through it.

### 6. Verify end to end

```bash
bash SKILL_DIR/scripts/doctor.sh [--plugin-dir <path>] [--port 1994] --test
```

`--test` starts a temporary server (only when the port is free), calls `get_metadata` through `POST /rpc`, then shuts it down. Success looks like `BRIDGE: connected` followed by Figma file data.

Interpret literally:
- `BRIDGE: connected` → working end to end.
- `BRIDGE: plugin not connected` → step 5 incomplete, or the plugin points at another port.
- `REGISTERED_OLD:` → a client config still points at a figma-mcp binary outside the skill folder (an old copied install); go back to step 4 to re-register.
- `LEADER: nothing listening` without `--test` → inconclusive, not a failure; re-run with `--test`.
- `LEADER: up` with a version older than the release just installed, or `--test` refusing to start because the port is busy → a stale process owns the port; go back to step 3.

When Figma MCP tools are already live in the session, call `get_metadata` directly instead of probing over HTTP.

Close by reporting the binary path, manifest path, config file touched, and the user's remaining actions.

## Hard rules

- Never claim the bridge, tools, or install work without the corresponding script line as evidence. Unverified steps must be reported as unverified.
- Pop an `ask_user` dialog only after announcing the operation it asks about, and move on only after the user's explicit answer — never fire a dialog the user has not been prepared for.
- Never invoke the binary directly in the foreground — its stdio transport blocks forever. Use `doctor.sh --test`, which holds stdin through a FIFO and cleans up.
- Never kill figma-mcp processes with ad-hoc `pkill`/`killall`/`taskkill`: matching on the string `figma-mcp` also hits this skill's own scripts and unrelated servers such as `figma-mcp-go`. Use `cleanup.sh`.
- Never report a stale process as cleaned without the script's `STATUS: cleaned` plus the `PORT:` line.
- Never hand-edit MCP configs when `register_client.cjs` can do it; it creates `.bak` backups.
- Use absolute paths in MCP configs; clients do not expand `~`.
- The server binary runs in place from `SKILL_DIR/bin/` — never copy it to another directory, and never register a `command` path outside the skill folder. The plugin, by contrast, is always deployed OUT of the skill folder by `install.sh` — never import it from inside `SKILL_DIR`.
- Never move or delete the skill folder or the deployed plugin directory after setup: the MCP command points into the skill folder, and the imported plugin points at the deployed copy.
- Non-default port: pass `--port N` in the config `args` **and** set the same port in the plugin gear dialog, otherwise the bridge stays down.
- Do not suggest building from source or `npx` — the source repo is private and no npm package exists.
- Invent no tool names, flags, endpoints, or menu paths beyond those in this file and `references/troubleshooting.md`.

## Troubleshooting

Read `references/troubleshooting.md` for the symptom → cause → fix table, plugin import details, Windows manual setup, env vars (`FIGMA_MCP_TIMEOUT`, `FIGMA_MCP_TIMEOUT_DESIGN_CONTEXT`, election jitter, `RUST_LOG`), and manual config shapes.
