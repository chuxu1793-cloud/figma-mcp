use rmcp::model::Tool;
use serde_json::json;

use crate::tool_helpers::*;

/// Write-style tools: create_paint_style, create_text_style, create_effect_style, create_grid_style, update_paint_style, delete_style, apply_style_to_node, set_effects, bind_variable_to_node (9 tools)
pub fn write_style_tools() -> Vec<Tool> {
    vec![
        tool("create_paint_style", "Create a new local paint style with a solid fill color.",
            schema_mixed(&[
                ("name", s("Style name e.g. 'Brand/Primary'"), true),
                ("color", s("Fill color as hex e.g. #FF5733"), true),
                ("description", s("Optional style description"), false),
            ])),

        tool("create_text_style", "Create a new local text style (typography preset). Returns the new style's ID. Apply it to nodes with apply_style_to_node. Use get_styles to list existing text styles.",
            schema_mixed(&[
                ("name", s("Style name — use slash notation to organise into groups e.g. 'Heading/H1', 'Body/Regular'"), true),
                ("fontSize", n("Font size in pixels (default 16)"), false),
                ("fontFamily", s("Font family name e.g. 'Inter', 'Roboto' (default Inter). Must be installed in Figma."), false),
                ("fontStyle", s("Font style variant e.g. 'Regular', 'Bold', 'Medium', 'SemiBold' (default Regular)"), false),
                ("textDecoration", s("Text decoration: NONE (default), UNDERLINE, or STRIKETHROUGH"), false),
                ("lineHeightValue", n("Line height value (unit set by lineHeightUnit)"), false),
                ("lineHeightUnit", s("Line height unit: PIXELS (default) or PERCENT"), false),
                ("letterSpacingValue", n("Letter spacing value (unit set by letterSpacingUnit)"), false),
                ("letterSpacingUnit", s("Letter spacing unit: PIXELS (default) or PERCENT"), false),
                ("description", s("Optional human-readable description shown in the Figma style panel"), false),
            ])),

        tool("create_effect_style", "Create a new local effect style (drop shadow, inner shadow, or blur).",
            schema_mixed(&[
                ("name", s("Style name e.g. 'Shadow/Card'"), true),
                ("type", s("Effect type: DROP_SHADOW (default), INNER_SHADOW, LAYER_BLUR, or BACKGROUND_BLUR"), false),
                ("color", s("Shadow color as hex e.g. #000000 (default #000000, shadows only)"), false),
                ("opacity", n("Shadow color opacity 0–1 (default 0.25, shadows only)"), false),
                ("radius", n("Blur radius in pixels (default 8 for shadows, 4 for blurs)"), false),
                ("offsetX", n("Shadow X offset in pixels (default 0, shadows only)"), false),
                ("offsetY", n("Shadow Y offset in pixels (default 4, shadows only)"), false),
                ("spread", n("Shadow spread in pixels (default 0, shadows only)"), false),
                ("description", s("Optional style description"), false),
            ])),

        tool("create_grid_style", "Create a new local layout grid style.",
            schema_mixed(&[
                ("name", s("Style name e.g. 'Grid/Desktop'"), true),
                ("pattern", s("Grid pattern: GRID (default), COLUMNS, or ROWS"), false),
                ("count", n("Number of columns or rows (COLUMNS/ROWS only, default 12)"), false),
                ("gutterSize", n("Gutter size in pixels (COLUMNS/ROWS only, default 16)"), false),
                ("offset", n("Margin/offset in pixels (COLUMNS/ROWS only, default 0)"), false),
                ("alignment", s("Alignment: STRETCH (default), CENTER, MIN, or MAX (COLUMNS/ROWS only)"), false),
                ("sectionSize", n("Grid cell size in pixels (GRID only, default 8)"), false),
                ("color", s("Grid line color as hex e.g. #FF0000 (GRID only, default #FF0000)"), false),
                ("opacity", n("Grid line opacity 0–1 (GRID only, default 0.1)"), false),
                ("description", s("Optional style description"), false),
            ])),

        tool("update_paint_style", "Update an existing paint style's name, color, or description. Only paint styles support in-place updates — to modify text, effect, or grid styles, use delete_style and recreate them.",
            schema_mixed(&[
                ("styleId", s("Paint style ID"), true),
                ("name", s("New style name"), false),
                ("color", s("New fill color as hex e.g. #FF5733"), false),
                ("description", s("New style description"), false),
            ])),

        tool("delete_style", "Delete a style (paint, text, effect, or grid) by its ID.",
            schema_mixed(&[("styleId", s("Style ID to delete"), true)])),

        tool("apply_style_to_node", "Apply an existing local style (paint, text, effect, or grid) to a node, linking the node to that style.",
            schema_mixed(&[
                ("nodeId", s("Target node ID in colon format e.g. 4029:12345"), true),
                ("styleId", s("Style ID to apply (from get_styles)"), true),
                ("target", s("For paint styles only — apply to 'fill' (default) or 'stroke'"), false),
            ])),

        tool("set_effects", "Apply one or more effects (drop shadow, inner shadow, layer blur, background blur) directly to a node. Replaces all existing effects. Pass an empty array to clear all effects.",
            schema_mixed(&[
                ("nodeId", s("Target node ID in colon format e.g. 4029:12345"), true),
                ("effects", arr_o("Array of effect objects. Each has: type (DROP_SHADOW | INNER_SHADOW | LAYER_BLUR | BACKGROUND_BLUR), radius, color (hex, shadows only), opacity (0–1, shadows only), offsetX, offsetY (shadows only), spread (shadows only), visible (default true)", json!({"type": "object"})), true),
            ])),

        tool("bind_variable_to_node", "Bind a local variable to a node property so the property is driven by the variable's value. COLOR variables: use fillColor or strokeColor. BOOLEAN variables: use visible. FLOAT variables: use opacity, rotation, width, height, cornerRadius, topLeftRadius, topRightRadius, bottomLeftRadius, bottomRightRadius, strokeWeight, itemSpacing, paddingTop, paddingRight, paddingBottom, paddingLeft.",
            schema_mixed(&[
                ("nodeId", s("Target node ID in colon format e.g. 4029:12345"), true),
                ("variableId", s("Variable ID to bind (from get_variable_defs)"), true),
                ("field", s("Property to bind: fillColor | strokeColor | visible | opacity | rotation | width | height | cornerRadius | topLeftRadius | topRightRadius | bottomLeftRadius | bottomRightRadius | strokeWeight | itemSpacing | paddingTop | paddingRight | paddingBottom | paddingLeft"), true),
            ])),
    ]
}
