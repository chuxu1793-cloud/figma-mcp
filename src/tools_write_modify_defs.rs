use rmcp::model::Tool;
use serde_json::json;

use crate::tool_helpers::*;

/// Write-modify tools: set_text, set_fills, set_strokes, move_nodes, resize_nodes,
/// rename_node, clone_node, set_opacity, set_corner_radius, set_auto_layout,
/// delete_nodes, set_visible, set_locked, rotate_nodes, reorder_nodes,
/// set_blend_mode, set_constraints, reparent_nodes, batch_rename_nodes,
/// find_replace_text, set_text_properties, set_gradient_fill, set_viewport,
/// set_plugin_data, set_text_range (25 tools)
pub fn write_modify_tools() -> Vec<Tool> {
    vec![
        tool("set_text", "Update the text content of an existing TEXT node.",
            schema_mixed(&[
                ("nodeId", s("TEXT node ID in colon format e.g. '4029:12345'"), true),
                ("text", s("New text content"), true),
            ])),

        tool("set_fills", "Set the fill color on one or more nodes. Use mode='append' to stack a new fill on top of existing fills instead of replacing them.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("color", s("Fill color as hex: #RRGGBB e.g. #FF5733 or #RRGGBBAA e.g. #FF573380 for 50% alpha"), true),
                ("opacity", n("Fill opacity 0–1 (default 1). Combines multiplicatively with any alpha in the color hex."), false),
                ("mode", s("'replace' (default) overwrites all existing fills; 'append' stacks this fill on top of existing ones"), false),
            ])),

        tool("set_strokes", "Set the stroke color and optionally the stroke weight on one or more nodes. Use mode='append' to stack a new stroke on top of existing strokes instead of replacing them. Omit strokeWeight to change only the color without affecting the weight.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("color", s("Stroke color as hex e.g. #000000"), true),
                ("strokeWeight", n("Stroke weight in pixels. Omit to keep existing weight."), false),
                ("mode", s("'replace' (default) overwrites all strokes; 'append' stacks on top of existing strokes"), false),
            ])),

        tool("move_nodes", "Move one or more nodes to an absolute canvas position. The same x/y is applied to every node independently (not a relative offset from current position).",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("x", n("Target X position"), false),
                ("y", n("Target Y position"), false),
            ])),

        tool("resize_nodes", "Resize one or more nodes. The same width/height is applied to every node in the list independently. Provide width, height, or both.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("width", n("New width in pixels"), false),
                ("height", n("New height in pixels"), false),
            ])),

        tool("rename_node", "Rename a single node by ID. Returns the updated node with its new name. Use batch_rename_nodes to rename multiple nodes at once or to apply find/replace patterns across many nodes.",
            schema_mixed(&[
                ("nodeId", s("Node ID in colon format e.g. '4029:12345'"), true),
                ("name", s("New name for the node. Figma supports slash-separated path notation e.g. 'Icons/Arrow/Left' to organise nodes in component panels."), true),
            ])),

        tool("clone_node", "Clone an existing node, optionally repositioning it or placing it in a new parent.",
            schema_mixed(&[
                ("nodeId", s("Source node ID in colon format e.g. '4029:12345'"), true),
                ("x", n("X position of the clone"), false),
                ("y", n("Y position of the clone"), false),
                ("parentId", s("Parent node ID for the clone. Defaults to same parent as source."), false),
            ])),

        tool("set_opacity", "Set the opacity of one or more nodes (0 = fully transparent, 1 = fully opaque).",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("opacity", n("Opacity value between 0 and 1"), true),
            ])),

        tool("set_corner_radius", "Set corner radius on one or more nodes. Provide a uniform cornerRadius or individual per-corner values.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("cornerRadius", n("Uniform corner radius applied to all corners"), false),
                ("topLeftRadius", n("Top-left corner radius"), false),
                ("topRightRadius", n("Top-right corner radius"), false),
                ("bottomLeftRadius", n("Bottom-left corner radius"), false),
                ("bottomRightRadius", n("Bottom-right corner radius"), false),
            ])),

        tool("set_auto_layout", "Set or update auto-layout (flex) properties on an existing frame.",
            schema_mixed(&[
                ("nodeId", s("Frame node ID in colon format e.g. '4029:12345'"), true),
                ("layoutMode", s("Auto-layout direction: HORIZONTAL, VERTICAL, or NONE"), false),
                ("paddingTop", n("Top padding"), false),
                ("paddingRight", n("Right padding"), false),
                ("paddingBottom", n("Bottom padding"), false),
                ("paddingLeft", n("Left padding"), false),
                ("itemSpacing", n("Gap between children"), false),
                ("primaryAxisAlignItems", s("Main-axis alignment: MIN, CENTER, MAX, or SPACE_BETWEEN"), false),
                ("counterAxisAlignItems", s("Cross-axis alignment: MIN, CENTER, MAX, or BASELINE"), false),
                ("primaryAxisSizingMode", s("Main-axis sizing: FIXED or AUTO (hug)"), false),
                ("counterAxisSizingMode", s("Cross-axis sizing: FIXED or AUTO (hug)"), false),
                ("layoutWrap", s("Wrap behaviour: NO_WRAP or WRAP"), false),
                ("counterAxisSpacing", n("Gap between wrapped rows/columns (only when layoutWrap is WRAP)"), false),
            ])),

        tool("delete_nodes", "Delete one or more nodes. This cannot be undone via MCP — use with care.",
            schema_mixed(&[("nodeIds", arr_s("Node IDs to delete in colon format e.g. ['4029:12345']"), true)])),

        tool("set_visible", "Show or hide one or more nodes by setting their visibility.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("visible", b("true to show the node, false to hide it"), true),
            ])),

        tool("set_locked", "Lock or unlock one or more nodes. Locked nodes cannot be accidentally edited in Figma.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("locked", b("true to lock, false to unlock"), true),
            ])),

        tool("rotate_nodes", "Rotate one or more nodes to an absolute angle in degrees.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("rotation", n("Rotation angle in degrees (positive = counter-clockwise in Figma)"), true),
            ])),

        tool("reorder_nodes", "Change the z-order (layer stack position) of one or more nodes.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("order", s("Order operation: bringToFront, sendToBack, bringForward, or sendBackward"), true),
            ])),

        tool("set_blend_mode", "Set the blend mode of one or more nodes (e.g. MULTIPLY, SCREEN, OVERLAY).",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("blendMode", s("Blend mode: NORMAL, MULTIPLY, SCREEN, OVERLAY, DARKEN, LIGHTEN, COLOR_DODGE, COLOR_BURN, HARD_LIGHT, SOFT_LIGHT, DIFFERENCE, EXCLUSION, HUE, SATURATION, COLOR, LUMINOSITY, PASS_THROUGH"), true),
            ])),

        tool("set_constraints", "Set layout constraints (pinning behaviour) on one or more nodes relative to their parent.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("horizontal", s("Horizontal constraint: MIN (left), MAX (right), CENTER, STRETCH, or SCALE"), false),
                ("vertical", s("Vertical constraint: MIN (top), MAX (bottom), CENTER, STRETCH, or SCALE"), false),
            ])),

        tool("reparent_nodes", "Move one or more nodes to a different parent frame, group, or section.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs to move in colon format e.g. ['4029:12345']"), true),
                ("parentId", s("Target parent node ID in colon format e.g. '4029:99'"), true),
            ])),

        tool("batch_rename_nodes", "Rename multiple nodes using find/replace, regex substitution, or prefix/suffix addition.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("find", s("String (or regex pattern when useRegex=true) to search for in the node name"), false),
                ("replace", s("Replacement string. Required when find is provided."), false),
                ("useRegex", b("Treat find as a regular expression (default false)"), false),
                ("regexFlags", s("Regex flags e.g. 'gi' (default 'g'). Only used when useRegex=true."), false),
                ("prefix", s("String to prepend to the node name"), false),
                ("suffix", s("String to append to the node name"), false),
            ])),

        tool("find_replace_text", "Find and replace text content across all TEXT nodes in a subtree. Searches the entire current page if no nodeId is given.",
            schema_mixed(&[
                ("find", s("Text string (or regex pattern when useRegex=true) to search for"), true),
                ("replace", s("Replacement string (use empty string to delete matches)"), true),
                ("nodeId", s("Root node ID to scope the search. Defaults to the entire current page."), false),
                ("useRegex", b("Treat find as a regular expression (default false)"), false),
                ("regexFlags", s("Regex flags e.g. 'gi' (default 'g'). Only used when useRegex=true."), false),
            ])),

        tool("set_text_properties", "Modify typography properties (fontSize, fontFamily, fontStyle, lineHeight, letterSpacing, textDecoration, textCase) on an existing TEXT node. Only provided properties are changed; omitted ones are left unchanged. The font must be installed in Figma.",
            schema_mixed(&[
                ("nodeId", s("TEXT node ID in colon format e.g. '4029:12345'"), true),
                ("fontSize", n("Font size in pixels"), false),
                ("fontFamily", s("Font family name e.g. 'Inter', 'Roboto'. Must be installed in Figma."), false),
                ("fontStyle", s("Font style variant e.g. 'Regular', 'Bold', 'Medium', 'SemiBold'"), false),
                ("lineHeightValue", n("Line height value (unit set by lineHeightUnit)"), false),
                ("lineHeightUnit", s("Line height unit: PIXELS or PERCENT or AUTO"), false),
                ("letterSpacingValue", n("Letter spacing value (unit set by letterSpacingUnit)"), false),
                ("letterSpacingUnit", s("Letter spacing unit: PIXELS or PERCENT"), false),
                ("textDecoration", s("Text decoration: NONE, UNDERLINE, or STRIKETHROUGH"), false),
                ("textCase", s("Text case: ORIGINAL, UPPER, LOWER, TITLE, or SMALL_CAPS"), false),
                ("textAlignHorizontal", s("Horizontal alignment: LEFT, CENTER, RIGHT, or JUSTIFIED"), false),
                ("textAlignVertical", s("Vertical alignment: TOP, CENTER, or BOTTOM"), false),
            ])),

        tool("set_gradient_fill", "Set a gradient fill on one or more nodes. Supports linear, radial, diamond, and angular gradient types. Use mode='append' to stack the gradient on top of existing fills.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs in colon format e.g. ['4029:12345']"), true),
                ("gradientType", s("Gradient type: GRADIENT_LINEAR, GRADIENT_RADIAL, GRADIENT_DIAMOND, or GRADIENT_ANGULAR"), true),
                ("stops", arr_o("Array of gradient stops", json!({
                    "type": "object",
                    "properties": {
                        "color": {"type": "string", "description": "Stop color as hex e.g. #FF5733"},
                        "position": {"type": "number", "description": "Position along gradient 0–1"},
                        "opacity": {"type": "number", "description": "Stop opacity 0–1 (default 1)"}
                    },
                    "required": ["color", "position"]
                })), true),
                ("gradientTransform", s("Optional 2x3 transform matrix as JSON array e.g. [[1,0,0],[0,1,0]]"), false),
                ("mode", s("'replace' (default) or 'append'"), false),
            ])),

        tool("set_viewport", "Control the Figma viewport — zoom, pan to a center point, or scroll to a specific node. Useful for programmatic navigation and screenshots.",
            schema_mixed(&[
                ("zoom", n("Zoom level (e.g. 1.0 = 100%)"), false),
                ("center", s("Center point as JSON: {\"x\": 0, \"y\": 0}"), false),
                ("scrollTo", s("Node ID to scroll and zoom into, colon format"), false),
            ])),

        tool("set_plugin_data", "Store custom plugin data on a node or the current page. Data is accessible via get_plugin_data. Use scope='shared' for sharedPluginData (accessible by other plugins).",
            schema_mixed(&[
                ("nodeId", s("Node ID to store data on, colon format. Defaults to current page."), false),
                ("key", s("Data key"), true),
                ("value", s("Data value (string). Pass empty string to delete."), true),
                ("scope", s("Data scope: 'plugin' (default) or 'shared'"), false),
            ])),

        tool("set_text_range", "Format a character range within a TEXT node. Allows setting fontSize, fillColor, fontFamily, fontStyle, and textDecoration on a subset of characters (e.g. partial bold). The font must be installed in Figma.",
            schema_mixed(&[
                ("nodeId", s("TEXT node ID in colon format e.g. '4029:12345'"), true),
                ("start", n("Start character index (0-based, default 0)"), false),
                ("end", n("End character index (exclusive, default = text length)"), false),
                ("fontSize", n("Font size in pixels for this range"), false),
                ("fillColor", s("Text color as hex for this range e.g. #FF5733"), false),
                ("fontFamily", s("Font family for this range"), false),
                ("fontStyle", s("Font style for this range e.g. 'Bold', 'Italic'"), false),
                ("textDecoration", s("Text decoration: NONE, UNDERLINE, or STRIKETHROUGH"), false),
            ])),
    ]
}
