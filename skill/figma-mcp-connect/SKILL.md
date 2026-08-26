---
name: figma-mcp-connect
description: Get the figma-mcp server (Rust Figma MCP with plugin bridge, no Figma API token) installed, registered in an MCP client, connected to the Figma plugin, verified, and unblocked. Use when the user wants to install, update, configure, start, or connect figma-mcp in Codely / Claude / Cursor / VS Code, import or open the Figma plugin, or when Figma MCP tools fail with errors like "plugin not connected", "request timed out", tools missing after config, Gatekeeper blocking the binary, or port 1994 conflicts. Triggers on phrasings like "安装 figma-mcp", "配置 figma mcp", "启动 figma mcp", "figma mcp 连不上", "插件没连上", "install figma-mcp", "figma mcp not working".
---

# figma-mcp connect

Automate everything that can be automated; ask the user only for the Figma GUI steps.

## Facts that drive the workflow

- Distribution: prebuilt binaries at `github.com/chuxu1793-cloud/figma-mcp` releases. No source build, no npm package, no token needed.
- The binary is an MCP **stdio** server — the MCP client spawns it on demand. Never "start it as a service"; nothing listening between sessions is normal.
- Each process also serves `127.0.0.1:1994` (`GET /ping`, `POST /rpc`, `GET /ws`) and elects a leader by binding the port.
- The Figma plugin auto-connects on open and retries every 1.5s. Importing/opening it is GUI-only and **cannot** be scripted.

## Workflow

Run `scripts/*` with absolute paths from this skill directory. All scripts print `KEY: value` lines; `STATUS: error` plus `REASON:` means failure.

### 1. Assess before acting

```bash
bash <skill>/scripts/doctor.sh [--dir ~/figma] [--port 1994]
```

Read `NEXT:` lines and do only what is missing. Skip to step 4 if everything is present.

### 2. Install or update

```bash
bash <skill>/scripts/install.sh [--dir ~/figma] [--version v0.1.1] [--force]
```

Idempotent: verifies sha256 against the release `SHA256SUMS.txt`, reports `already-current` when the local binary matches, strips the macOS quarantine attribute, and unpacks the plugin to `<dir>/plugin/`. Use `--force` to reinstall or downgrade.

### 3. Register in the MCP client

```bash
node <skill>/scripts/register_client.cjs --client <id> --binary <dir>/figma-mcp [--name figma]
```

`--client` ids: `codely`, `claude-desktop`, `claude-code` (project `.mcp.json`), `cursor`, `cursor-project`, `vscode` (project `.vscode/mcp.json`). Add `--config <path>` for a non-default location.

Pick the id from context — when running inside Codely CLI and the user names no other tool, use `codely`. Ask only if the target is genuinely ambiguous. The script backs up the file, writes the right shape per client, and is idempotent.

If it reports a JSON parse error (comments in `.vscode/mcp.json` are common), edit the file manually using the shapes in `references/troubleshooting.md`.

Then tell the user to restart the client session — MCP config is read at startup.

### 4. Bring up the Figma side

Automate the openable parts, then hand off:

```bash
open -a Figma                    # macOS: launch Figma Desktop
open <dir>/plugin                # reveal the folder holding manifest.json
```

Then instruct the user with these exact steps (first time only):

1. **Plugins → Development → Import plugin from manifest…**
2. Select `<dir>/plugin/manifest.json`
3. Open a Figma file, then **Plugins → Development → Figma MCP**
4. Keep the plugin window open — closing it drops the bridge

State plainly that Figma has no automation hook for these steps, and that a browser tab will not work (Desktop app required).

### 5. Verify end to end

```bash
bash <skill>/scripts/doctor.sh --dir <dir> --test
```

`--test` starts a temporary server (only when the port is free), calls `get_metadata` through `POST /rpc`, and shuts it down. Expect `BRIDGE: connected` plus the Figma file name. `BRIDGE: plugin not connected` means step 4 is incomplete or the plugin points at another port.

When Figma MCP tools are already live in the session, prefer calling `get_metadata` directly instead of probing over HTTP.

Report back: binary path, manifest path, config file touched, and any remaining user action.

## Hard rules

- Never invoke the binary directly in the foreground — its stdio transport blocks forever. Use `doctor.sh --test`, which holds stdin through a FIFO and cleans up.
- Never hand-edit MCP configs when `register_client.cjs` can do it; it creates `.bak` backups.
- Use absolute paths in MCP configs; clients do not expand `~`.
- Non-default port: pass `--port N` in the config `args` **and** set the same port in the plugin gear dialog, otherwise the bridge stays down.
- Do not suggest building from source or `npx` — the source repo is private and no npm package exists.

## Troubleshooting

Read `references/troubleshooting.md` for the symptom → cause → fix table, plugin import details, env vars (`FIGMA_MCP_TIMEOUT`, `FIGMA_MCP_TIMEOUT_DESIGN_CONTEXT`, election jitter, `RUST_LOG`), and manual config shapes.
