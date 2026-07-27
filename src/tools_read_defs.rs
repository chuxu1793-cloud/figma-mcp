use rmcp::model::Tool;

use crate::tool_helpers::*;

/// Returns all 19 read tool definitions.
pub fn read_tools() -> Vec<Tool> {
    vec![
        tool("get_document", "Get the full node tree of the current page (not the whole file — only the active page). Returns all nodes recursively and can be very large. Prefer get_design_context for exploration or when token efficiency matters.", no_params_schema()),

        tool("get_pages", "List all pages in the document with their IDs and names. Lightweight alternative to get_document.", no_params_schema()),

        tool("get_metadata", "Get metadata about the current Figma document: file name, pages, current page", no_params_schema()),

        tool("get_selection", "Get the nodes currently selected in Figma. Returns an empty array if nothing is selected. Use get_design_context or get_node to retrieve deeper detail about a specific node by ID.", no_params_schema()),

        tool("get_node", "Get a single node by ID with full detail. Use get_nodes_info to fetch multiple nodes in one round-trip instead of calling this repeatedly. Node ID must be colon format e.g. '4029:12345', never hyphens.",
            schema(&[("nodeId", "string", true, "Node ID in colon format e.g. '4029:12345'")])),

        tool("get_nodes_info", "Get full details for multiple nodes by ID in one round-trip. Prefer this over calling get_node repeatedly when you need several nodes.",
            schema_mixed(&[("nodeIds", arr_s("List of node IDs in colon format e.g. ['4029:12345', '4029:67890']"), true)])),

        tool("get_design_context", "Get a depth-limited, token-efficient tree of the current selection or page. Use this instead of get_document when exploring large files. Supports detail levels (minimal/compact/full) and dedupe_components for pages heavy with repeated component instances.",
            schema_mixed(&[
                ("depth", n("How many levels deep to traverse (default 2)"), false),
                ("detail", s("Property verbosity: minimal (id/name/type/bounds only), compact (+fills/strokes/opacity), full (everything, default)"), false),
                ("dedupe_components", b("When true, INSTANCE nodes are serialized compactly (mainComponentId + componentProperties + overrides array of differing text/nested content) and unique component definitions are collected once in a top-level componentDefs map. Highly token-efficient for screens with many repeated component instances."), false),
            ])),

        tool("search_nodes", "Search for nodes by name substring and/or type within a subtree. Use this when you know (part of) the node name. Use scan_nodes_by_types when you want all nodes of a type regardless of name.",
            schema_mixed(&[
                ("query", s("Name substring to match (case-insensitive)"), true),
                ("nodeId", s("Scope search to this subtree (default: current page), colon format e.g. '4029:12345'"), false),
                ("types", arr_s("Filter by Figma node type e.g. ['TEXT', 'FRAME', 'COMPONENT']"), false),
                ("limit", n("Maximum results to return (default: 50)"), false),
            ])),

        tool("scan_text_nodes", "Scan all TEXT nodes in a subtree and return their content. Shorthand for scan_nodes_by_types with ['TEXT'] — use when you only need text copy from a component or frame.",
            schema(&[("nodeId", "string", true, "Root node ID to scan from, colon format e.g. '4029:12345'")])),

        tool("scan_nodes_by_types", "Find all nodes of specific types in a subtree, regardless of name. Use search_nodes instead when you need to filter by name.",
            schema_mixed(&[
                ("nodeId", s("Root node ID to scan from, colon format e.g. '4029:12345'"), true),
                ("types", arr_s("Node types to find e.g. ['FRAME', 'COMPONENT', 'INSTANCE']"), true),
            ])),

        tool("get_reactions", "Get the prototype reactions defined on a node. Returns an array of reaction objects — each has a trigger (e.g. ON_CLICK, ON_HOVER, AFTER_TIMEOUT) and an actions array (navigate to node, open URL, go back, etc.). Use set_reactions to add or replace reactions, remove_reactions to delete them.",
            schema(&[("nodeId", "string", true, "Node ID in colon format e.g. '4029:12345'")])),

        tool("get_viewport", "Get the current Figma viewport: scroll center, zoom level, and visible bounds.", no_params_schema()),

        tool("get_fonts", "List all fonts used in the current page, sorted by usage frequency. Useful for understanding typography without scanning all text nodes.", no_params_schema()),

        tool("get_styles", "Get all local styles in the document (paint, text, effect, and grid). Returns each style's ID, name, type, and properties. Use the style ID with apply_style_to_node or update_paint_style. For design tokens (variables), use get_variable_defs instead.", no_params_schema()),

        tool("get_variable_defs", "Get all local variable definitions: collections, modes, and values. Variables are Figma's design token system.", no_params_schema()),

        tool("get_local_components", "Get all components defined in the current Figma file.", no_params_schema()),

        tool("get_annotations", "Get dev-mode annotations in the current document or scoped to a specific node. Returns annotation objects with label text, measurement type, and the ID of the annotated node. Omit nodeId to retrieve all annotations on the current page.",
            schema_mixed(&[("nodeId", s("Optional — scope results to annotations on this node and its descendants, colon format e.g. '4029:12345'"), false)])),

        tool("export_tokens", "Export all design tokens (variables and paint styles) as JSON or CSS custom properties. Ideal for bridging Figma variables into your codebase.",
            schema_mixed(&[("format", s("Output format: json (default) or css"), false)])),

        tool("get_screenshot", "Export a screenshot of one or more nodes as base64-encoded image data (held in memory). Use save_screenshots instead when you want to write images directly to disk without base64 in the response.",
            schema_mixed(&[
                ("nodeIds", arr_s("Optional node IDs to export, colon format. If empty, exports current selection."), false),
                ("format", s("Export format: PNG (default), SVG, JPG, or PDF"), false),
                ("scale", n("Export scale for raster formats (default 2)"), false),
            ])),
    ]
}
