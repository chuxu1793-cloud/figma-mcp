---
name: figma-mcp-connect
description: Get the figma-mcp server (Rust Figma MCP with plugin bridge, no Figma API token) installed, registered in an MCP client, connected to the Figma plugin, verified, and unblocked. Use when the user wants to install, update, configure, start, or connect figma-mcp in Codely / Claude / Cursor / VS Code, import or open the Figma plugin, or when Figma MCP tools fail with errors like "plugin not connected", "request timed out", tools missing after config, Gatekeeper blocking the binary, or port 1994 conflicts. Triggers on phrasings like "安装 figma-mcp", "配置 figma mcp", "启动 figma mcp", "figma mcp 连不上", "插件没连上", "install figma-mcp", "figma mcp not working".
---

# figma-mcp connect

Automate everything that can be automated; ask the user only for the Figma GUI steps.

`SKILL_DIR` below = the absolute directory containing this SKILL.md (shown in the activation notice). Never guess it, and never retype script contents — call the scripts.

## Facts that drive the workflow

- Distribution: prebuilt binaries from GitHub releases of `chuxu1793-cloud/figma-mcp`. No source build, no npm package, no Figma API token. Prebuilt targets: darwin-arm64, darwin-amd64, linux-amd64, windows-amd64.
- The binary is an MCP **stdio** server — the MCP client spawns it on demand. Never "start it as a service"; nothing listening between sessions is normal.
- Each process also serves `127.0.0.1:1994` (`GET /ping`, `POST /rpc`, `GET /ws`) and elects a leader by binding the port.
- The Figma plugin auto-connects on open and retries every 1.5s. Importing/opening it is GUI-only and **cannot** be scripted.

## Platform matrix — check the OS before choosing commands

| | macOS | Windows | Linux |
|---|---|---|---|
| `install.sh` / `doctor.sh` | native | needs Git Bash or WSL (auto-detects MSYS/Cygwin, uses the `.exe` asset, falls back to PowerShell `Expand-Archive` when `unzip` is absent) | native |
| `register_client.cjs` | native | native (`node` on Windows works; Claude Desktop path resolves to `%APPDATA%\Claude`) | native |
| Launch Figma | `open -a Figma` | Start menu — no reliable CLI hook; do not invent one | **no official Figma Desktop app** → plugin bridge unavailable |
| Reveal plugin folder | `open <dir>/plugin` | `explorer.exe <dir>\plugin` (Git Bash) or `explorer <dir>\plugin` (cmd/PowerShell) | `xdg-open <dir>/plugin` |

Determine the OS from the environment context or `uname -s` before emitting any command. On Linux, install and register normally, but state up front that importing the plugin requires the Figma Desktop app, which Figma does not ship for Linux — the server will run without a bridge. If Windows has no Git Bash or WSL, follow the manual sequence in `references/troubleshooting.md` instead of inventing shell commands.

## Workflow

All scripts print `KEY: value` lines; `STATUS: error` plus `REASON:` means failure. Report those lines back rather than paraphrasing them.

### 1. Assess before acting

```bash
bash SKILL_DIR/scripts/doctor.sh [--dir ~/figma] [--port 1994]
```

Act only on the `NEXT:` lines it prints. Skip to step 4 if nothing else is missing.

### 2. Install or update

```bash
bash SKILL_DIR/scripts/install.sh [--dir ~/figma] [--version <tag>] [--force]
```

Idempotent: verifies sha256 of binary and plugin zip against the release `SHA256SUMS.txt`, reports `already-current` when the local binary matches, strips the macOS quarantine attribute, unpacks the plugin to `<dir>/plugin/`. `--force` reinstalls or downgrades.

Omit `--version` (defaults to the latest release) unless the user names a tag. Do not invent tags or version numbers, and do not state a version unless a script or `GET /ping` reported it — the binary has no `--version` flag.

### 3. Register in the MCP client

```bash
node SKILL_DIR/scripts/register_client.cjs --client <id> --binary <dir>/figma-mcp [--name figma]
```

Valid `--client` ids, exactly these: `codely`, `claude-desktop`, `claude-code` (project `.mcp.json`), `cursor`, `cursor-project`, `vscode` (project `.vscode/mcp.json`). Add `--config <path>` for a non-default location. Never pass an id outside this list; the script rejects unknown ids and prints the valid set.

Pick the id from context — when running inside Codely CLI and the user names no other tool, use `codely`. Ask only if the target is genuinely ambiguous. The script backs up the file, writes the right shape per client, and is idempotent.

If it reports a JSON parse error (comments in `.vscode/mcp.json` are common), edit the file manually using the shapes in `references/troubleshooting.md`.

Then tell the user to restart the client session — MCP config is read at startup.

### 4. Bring up the Figma side

Automate the openable parts using the row for the current OS in the platform matrix above — launch Figma Desktop where a CLI hook exists, and reveal the folder holding `manifest.json`.

Then give the user these exact steps (first time only):

1. **Plugins → Development → Import plugin from manifest…**
2. Select `<dir>/plugin/manifest.json`
3. Open a Figma file, then **Plugins → Development → Figma MCP**
4. Keep the plugin window open — closing it drops the bridge

State plainly that Figma exposes no automation hook for these steps, and that a browser tab will not work (Desktop app required). Do not claim to have imported or opened the plugin.

### 5. Verify end to end

```bash
bash SKILL_DIR/scripts/doctor.sh --dir <dir> --test
```

`--test` starts a temporary server (only when the port is free), calls `get_metadata` through `POST /rpc`, then shuts it down. Success looks like `BRIDGE: connected` followed by Figma file data.

Interpret literally:
- `BRIDGE: connected` → working end to end.
- `BRIDGE: plugin not connected` → step 4 incomplete, or the plugin points at another port.
- `LEADER: nothing listening` without `--test` → inconclusive, not a failure; re-run with `--test`.

When Figma MCP tools are already live in the session, call `get_metadata` directly instead of probing over HTTP.

Close by reporting the binary path, manifest path, config file touched, and the user's remaining actions.

## Hard rules

- Never claim the bridge, tools, or install work without the corresponding script line as evidence. Unverified steps must be reported as unverified.
- Never invoke the binary directly in the foreground — its stdio transport blocks forever. Use `doctor.sh --test`, which holds stdin through a FIFO and cleans up.
- Never hand-edit MCP configs when `register_client.cjs` can do it; it creates `.bak` backups.
- Use absolute paths in MCP configs; clients do not expand `~`.
- Non-default port: pass `--port N` in the config `args` **and** set the same port in the plugin gear dialog, otherwise the bridge stays down.
- Do not suggest building from source or `npx` — the source repo is private and no npm package exists.
- Invent no tool names, flags, endpoints, or menu paths beyond those in this file and `references/troubleshooting.md`.

## Troubleshooting

Read `references/troubleshooting.md` for the symptom → cause → fix table, plugin import details, Windows manual setup, env vars (`FIGMA_MCP_TIMEOUT`, `FIGMA_MCP_TIMEOUT_DESIGN_CONTEXT`, election jitter, `RUST_LOG`), and manual config shapes.
