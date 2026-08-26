# figma-mcp troubleshooting

Contents: architecture facts · symptom table · plugin import steps · env vars · manual config shapes

## Architecture facts that explain most failures

- The binary is an MCP **stdio** server. The MCP client spawns it; it is not a background service. Nothing listening on the port between sessions is normal.
- Each process also runs an HTTP/WebSocket server on `127.0.0.1:1994` (`--ip`, `--port` to change): `GET /ping`, `POST /rpc`, `GET /ws`.
- Multiple instances elect a **leader** by binding the port. Only the leader holds the WebSocket to the Figma plugin; followers proxy tool calls to `POST /rpc`. If the leader dies, a follower takes over within ~3–5s.
- The plugin connects to `ws://<host>:<port>/ws` **automatically** when opened, and retries every 1.5s. There is no Connect button; the gear icon only changes host/port.
- Host/port entered in the plugin persist via `figma.clientStorage`, not localStorage — they survive restarts and are per Figma user.

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
| Plugin missing from Figma's menu | Manifest not imported, or imported from a deleted path | Re-import `<dir>/plugin/manifest.json`; keep the directory in place |
| Import option greyed out / absent | Using Figma in a browser | Development plugins require the Figma **Desktop** app |
| `--version` fails with "unexpected argument" | Flag does not exist | Read the version from `GET /ping`, or compare the binary's sha256 against `SHA256SUMS.txt` |
| Linux arm64 has no asset | Only darwin-arm64, darwin-amd64, linux-amd64, windows-amd64 are built | Build from source, or run the amd64 build under emulation |

## Figma plugin import (GUI only — cannot be scripted)

1. Open the **Figma Desktop** app (`open -a Figma` on macOS).
2. Menu: **Plugins → Development → Import plugin from manifest…**
3. Select `<install-dir>/plugin/manifest.json` (default `~/figma/plugin/manifest.json`).
4. Open any Figma file, then **Plugins → Development → Figma MCP**.
5. The plugin window must stay open — closing it drops the WebSocket. It reconnects on its own when reopened.

One-time only: after step 3 the plugin stays in the Development menu.

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
