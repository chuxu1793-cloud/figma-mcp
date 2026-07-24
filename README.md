# figma-mcp

Figma MCP — Free, No Rate Limits (Rust Edition)
<p>
  <a href="https://www.npmjs.com/package/@tuanjie/figma-mcp"><img src="https://img.shields.io/npm/v/@tuanjie/figma-mcp?color=blue" alt="npm version" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT" /></a>
  <a href="https://github.com/tuanjie/figma-mcp/stargazers"><img src="https://img.shields.io/github/stars/tuanjie/figma-mcp?style=social" alt="GitHub stars" /></a>
</p>

Open-source Figma MCP server written in Rust, with full read/write access via plugin — no REST API, no rate limits. Turn text into designs and designs into real code. Works with Cursor, Claude, GitHub Copilot, and any MCP-compatible AI tool.

**Highlights**
- No Figma API token required
- No rate limits — free plan friendly
- **Read and Write** live Figma data via plugin bridge — 73 tools total
- Full design automation — styles, variables, components, prototypes, and content
- Design strategies included — 12 prompts built in
- Written in Rust — fast, memory-safe, single binary

---

## Installation & Setup

### 1. Configure your AI tool

**Claude Code CLI**
```bash
claude mcp add -s project figma-mcp -- npx -y @tuanjie/figma-mcp@latest
```

**Codex CLI**
```bash
codex mcp add figma-mcp -- npx -y @tuanjie/figma-mcp@latest
```

**.mcp.json** (Claude and other MCP-compatible tools)
```json
{
  "mcpServers": {
    "figma-mcp": {
      "command": "npx",
      "args": ["-y", "@tuanjie/figma-mcp"]
    }
  }
}
```

**.vscode/mcp.json** (Cursor / VS Code / GitHub Copilot)
```json
{
  "servers": {
    "figma-mcp": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@tuanjie/figma-mcp"]
    }
  }
}
```

### 2. Install the Figma plugin

1. In Figma Desktop: **Plugins → Development → Import plugin from manifest**
2. Select `manifest.json` from the [plugin.zip](https://github.com/tuanjie/figma-mcp/releases)
3. Run the plugin inside any Figma file

---

## Available Tools (73 total)

### Read Tools (19)

| Category | Tools |
|----------|-------|
| Document | `get_document`, `get_pages`, `get_metadata`, `get_selection`, `get_viewport` |
| Nodes | `get_node`, `get_nodes_info`, `get_design_context`, `search_nodes`, `scan_text_nodes`, `scan_nodes_by_types` |
| Styles | `get_styles`, `get_variable_defs`, `get_local_components`, `get_annotations`, `get_fonts` |
| Prototype | `get_reactions` |
| Export | `get_screenshot`, `save_screenshots`, `export_frames_to_pdf`, `export_tokens` |

### Write Tools (54)

| Category | Tools |
|----------|-------|
| Create | `create_frame`, `create_rectangle`, `create_ellipse`, `create_text`, `import_image`, `create_component`, `create_section` |
| Modify | `set_text`, `set_fills`, `set_strokes`, `move_nodes`, `resize_nodes`, `rename_node`, `clone_node`, `set_opacity`, `set_corner_radius`, `set_auto_layout`, `delete_nodes`, `set_visible`, `lock_nodes`, `unlock_nodes`, `rotate_nodes`, `reorder_nodes`, `set_blend_mode`, `set_constraints`, `reparent_nodes`, `batch_rename_nodes`, `find_replace_text` |
| Styles | `create_paint_style`, `create_text_style`, `create_effect_style`, `create_grid_style`, `update_paint_style`, `delete_style`, `apply_style_to_node`, `set_effects` |
| Variables | `create_variable_collection`, `add_variable_mode`, `create_variable`, `set_variable_value`, `delete_variable`, `bind_variable_to_node` |
| Components | `navigate_to_page`, `group_nodes`, `ungroup_nodes`, `swap_component`, `detach_instance` |
| Prototype | `set_reactions`, `remove_reactions` |
| Pages | `add_page`, `delete_page`, `rename_page` |

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
# Build
cargo build --release

# Run
./target/release/figma-mcp --ip 127.0.0.1 --port 1994

# Test
cargo test
```

---

## License

MIT
