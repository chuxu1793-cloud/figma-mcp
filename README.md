# figma-mcp

Figma MCP — Free, No Rate Limits (Rust Edition)
<p>
  <a href="https://github.com/chuxu1793-cloud/figma-mcp/releases"><img src="https://img.shields.io/github/v/release/chuxu1793-cloud/figma-mcp?color=blue" alt="release version" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT" /></a>
  <a href="https://github.com/chuxu1793-cloud/figma-mcp/stargazers"><img src="https://img.shields.io/github/stars/chuxu1793-cloud/figma-mcp?style=social" alt="GitHub stars" /></a>
</p>

Open-source Figma MCP server written in Rust, with full read/write access via plugin — no REST API, no rate limits. Turn text into designs and designs into real code. Works with Cursor, Claude, GitHub Copilot, and any MCP-compatible AI tool.

**Highlights**
- No Figma API token required
- No rate limits — free plan friendly
- **Read and Write** live Figma data via plugin bridge — 75 tools total
- Full design automation — styles, variables, components, prototypes, and content
- Design strategies included — 12 prompts built in
- Written in Rust — fast, memory-safe, single binary

---

## Installation & Setup

### 1. Install via install script (recommended)

```bash
curl -sL https://raw.githubusercontent.com/chuxu1793-cloud/figma-mcp/main/install.sh | bash
```

This will download the pre-compiled binary and Figma plugin to `~/figma/`.

To install to a custom directory:

```bash
curl -sL https://raw.githubusercontent.com/chuxu1793-cloud/figma-mcp/main/install.sh | bash -s -- /custom/path
```

### Or: Manual install

```bash
# Create install directory
mkdir -p ~/figma && cd ~/figma

# Download binary (macOS Apple Silicon example)
curl -L -o figma-mcp https://github.com/chuxu1793-cloud/figma-mcp/releases/latest/download/figma-mcp-darwin-arm64
chmod +x figma-mcp

# Download plugin
curl -L -o figma-plugin.zip https://github.com/chuxu1793-cloud/figma-mcp/releases/latest/download/figma-plugin.zip
unzip figma-plugin.zip -d plugin
```

Available binaries by platform:

| Platform | File |
|----------|------|
| macOS Apple Silicon | `figma-mcp-darwin-arm64` |
| macOS Intel | `figma-mcp-darwin-amd64` |
| Linux x86_64 | `figma-mcp-linux-amd64` |
| Windows x86_64 | `figma-mcp-windows-amd64.exe` |

### 2. Configure your AI tool

**Claude Code CLI**
```bash
claude mcp add -s project figma -- ~/figma/figma-mcp
```

**.mcp.json** (Claude and other MCP-compatible tools)
```json
{
  "mcpServers": {
    "figma": {
      "command": "/Users/YOUR_USERNAME/figma/figma-mcp"
    }
  }
}
```

**.vscode/mcp.json** (Cursor / VS Code / GitHub Copilot)
```json
{
  "servers": {
    "figma": {
      "type": "stdio",
      "command": "/Users/YOUR_USERNAME/figma/figma-mcp"
    }
  }
}
```

> Replace `/Users/YOUR_USERNAME/figma/figma-mcp` with the actual path where you installed the binary.

### 3. Install the Figma plugin

1. In Figma Desktop: **Plugins → Development → Import plugin from manifest**
2. Select `~/figma/plugin/manifest.json`
3. Run the plugin inside any Figma file

---

## Available Tools (75 total)

### Read Tools (16)

| Category | Tools |
|----------|-------|
| Document | `get_pages`, `get_metadata`, `get_selection`, `get_viewport` |
| Nodes | `get_nodes_info`, `get_design_context`, `search_nodes`, `scan_nodes_by_types` |
| Styles | `get_styles`, `get_variable_defs`, `get_local_components`, `get_annotations`, `get_fonts` |
| Prototype | `get_reactions` |
| Export | `get_screenshot`, `export_frames_to_pdf`, `export_tokens` |

### Write Tools (59)

| Category | Tools |
|----------|-------|
| Create | `create_frame`, `create_rectangle`, `create_ellipse`, `create_text`, `import_image`, `create_component`, `create_section`, `create_line`, `create_star`, `create_polygon` |
| Modify | `set_text`, `set_text_properties`, `set_fills`, `set_strokes`, `move_nodes`, `resize_nodes`, `rename_node`, `clone_node`, `set_opacity`, `set_corner_radius`, `set_auto_layout`, `delete_nodes`, `set_visible`, `set_locked`, `rotate_nodes`, `reorder_nodes`, `set_blend_mode`, `set_constraints`, `reparent_nodes`, `batch_rename_nodes`, `find_replace_text` |
| Styles | `create_paint_style`, `create_text_style`, `create_effect_style`, `create_grid_style`, `update_paint_style`, `update_text_style`, `update_effect_style`, `update_grid_style`, `delete_style`, `apply_style_to_node`, `set_effects`, `bind_variable_to_node` |
| Variables | `create_variable_collection`, `add_variable_mode`, `create_variable`, `set_variable_value`, `delete_variable` |
| Components | `group_nodes`, `ungroup_nodes`, `swap_component`, `detach_instance` |
| Prototype | `set_reactions`, `remove_reactions` |
| Pages | `navigate_to_page`, `add_page`, `delete_page`, `rename_page` |
| Export | `export_frames_to_pdf` |

### MCP Prompts (12)

`read_design_strategy`, `design_strategy`, `text_replacement_strategy`, `annotation_conversion_strategy`, `swap_overrides_instances`, `reaction_to_connector_strategy`, `style_audit_strategy`, `bulk_rename_strategy`, `design_token_generation_strategy`, `generate_color_palette`, `generate_type_scale`, `generate_component_variants`

---

## Architecture

```
┌─────────────┐     stdio      ┌──────────────┐     WebSocket     ┌──────────────┐
│  AI Client   │◄─────────────►│  figma-mcp   │◄─────────────────►│  Figma Plugin │
│ (Claude etc)│    MCP/JSON   │   (Rust)     │   ws://127.0.0.1  │  (TypeScript) │
└─────────────┘                └──────────────┘   :1994/ws        └──────────────┘
```

The Rust server acts as an MCP server over stdio, and maintains a WebSocket bridge to the Figma plugin. Multiple MCP server instances coordinate via a Leader/Follower election mechanism — only the Leader holds the WebSocket connection to the plugin.

---

## Building from Source

```bash
# Build Rust binary
cargo build --release

# Build Figma plugin
cd plugin && bun install && bun run build

# Test
cargo test
```

---

## License

MIT
