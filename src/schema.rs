use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

/// Matches Figma node IDs:
/// simple:   "4029:12345"
/// compound: "I2167:9091;186:1579;186:1745" (instances/variants)
static NODE_ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^I?\d+:\d+(;\d+:\d+)*$").expect("invalid node-id regex"));

/// Converts hyphen-format node IDs (LLM output artifact) to colon format.
/// "4029-12345" → "4029:12345". No-ops for already-valid or unrecognized strings.
pub fn normalize_node_id(s: &str) -> String {
    if s.contains('-') && !s.contains(':') {
        let normalized = s.replace('-', ":");
        if NODE_ID_PATTERN.is_match(&normalized) {
            return normalized;
        }
    }
    s.to_string()
}

/// Reports whether s is a valid Figma node ID.
pub fn valid_node_id(s: &str) -> bool {
    NODE_ID_PATTERN.is_match(s)
}

/// Validates an incoming RPC request against the tool's expected input shape.
/// Returns Err(message) on failure, Ok(()) if valid.
pub fn validate_rpc(tool: &str, node_ids: &[String], params: &Map<String, Value>) -> Result<(), String> {
    match tool {
        "get_nodes_info" => {
            if node_ids.is_empty() {
                return Err("nodeIds is required and must not be empty".into());
            }
            for id in node_ids {
                if !valid_node_id(id) {
                    return Err(format!("invalid nodeId: {} — must use colon format e.g. 4029:12345", id));
                }
            }
        }

        "export_frames_to_pdf" => {
            if node_ids.is_empty() {
                return Err("nodeIds is required and must not be empty".into());
            }
            for id in node_ids {
                if !valid_node_id(id) {
                    return Err(format!("invalid nodeId: {} — must use colon format e.g. 4029:12345", id));
                }
            }
        }

        "get_screenshot" => {
            for id in node_ids {
                if !valid_node_id(id) {
                    return Err(format!("invalid nodeId: {} — must use colon format e.g. 4029:12345", id));
                }
            }
            if let Some(format) = params.get("format").and_then(|v| v.as_str()) {
                if !valid_export_format(format) {
                    return Err(format!("format must be PNG, SVG, JPG, or PDF, got: {}", format));
                }
            }
        }

        "get_design_context" => {
            if let Some(depth) = params.get("depth").and_then(|v| v.as_f64()) {
                if depth < 0.0 {
                    return Err("depth must be a non-negative number".into());
                }
            }
            if let Some(detail) = params.get("detail").and_then(|v| v.as_str()) {
                if !detail.is_empty() && !["minimal", "compact", "full"].contains(&detail) {
                    return Err(format!("detail must be minimal, compact, or full, got: {}", detail));
                }
            }
        }

        "search_nodes" => {
            let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
            if query.is_empty() {
                return Err("query is required".into());
            }
            if let Some(node_id) = params.get("nodeId").and_then(|v| v.as_str()) {
                if !node_id.is_empty() && !valid_node_id(node_id) {
                    return Err(format!("nodeId must use colon format e.g. 4029:12345, got: {}", node_id));
                }
            }
            if let Some(limit) = params.get("limit").and_then(|v| v.as_f64()) {
                if limit <= 0.0 {
                    return Err("limit must be a positive number".into());
                }
            }
        }

        "get_reactions" => {
            require_single_node_id(node_ids)?;
        }

        "scan_nodes_by_types" => {
            let node_id = params.get("nodeId").and_then(|v| v.as_str()).unwrap_or("");
            if node_id.is_empty() {
                return Err("nodeId is required".into());
            }
            if !valid_node_id(node_id) {
                return Err(format!("nodeId must use colon format e.g. 4029:12345, got: {}", node_id));
            }
            let types = params.get("types").and_then(|v| v.as_array());
            if types.is_none_or(|t| t.is_empty()) {
                return Err("types must be a non-empty array".into());
            }
        }

        // ── Write tools ──

        "set_opacity" => {
            validate_node_ids(node_ids)?;
            let op = params.get("opacity").and_then(|v| v.as_f64()).ok_or("opacity is required")?;
            if !(0.0..=1.0).contains(&op) {
                return Err("opacity must be between 0 and 1".into());
            }
        }

        "set_corner_radius" => {
            validate_node_ids(node_ids)?;
            let has_uniform = params.contains_key("cornerRadius");
            let has_tl = params.contains_key("topLeftRadius");
            let has_tr = params.contains_key("topRightRadius");
            let has_bl = params.contains_key("bottomLeftRadius");
            let has_br = params.contains_key("bottomRightRadius");
            if !has_uniform && !has_tl && !has_tr && !has_bl && !has_br {
                return Err("at least one of cornerRadius, topLeftRadius, topRightRadius, bottomLeftRadius, or bottomRightRadius is required".into());
            }
        }

        "group_nodes" => {
            if node_ids.len() < 2 {
                return Err("nodeIds must contain at least 2 nodes to group".into());
            }
            validate_node_ids(node_ids)?;
        }

        "ungroup_nodes" => {
            if node_ids.is_empty() {
                return Err("nodeIds is required and must not be empty".into());
            }
            validate_node_ids(node_ids)?;
        }

        "navigate_to_page" => {
            let page_id = params.get("pageId").and_then(|v| v.as_str()).unwrap_or("");
            let page_name = params.get("pageName").and_then(|v| v.as_str()).unwrap_or("");
            if page_id.is_empty() && page_name.is_empty() {
                return Err("pageId or pageName is required".into());
            }
        }

        "create_component" => {
            require_single_node_id(node_ids)?;
        }

        "export_tokens" => {
            if let Some(format) = params.get("format").and_then(|v| v.as_str()) {
                if !format.is_empty() && !["json", "css"].contains(&format) {
                    return Err(format!("format must be json or css, got: {}", format));
                }
            }
        }

        "get_plugin_data" => {
            let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("");
            if key.is_empty() { return Err("key is required".into()); }
        }

        "export_nodes" => {
            for id in node_ids {
                if !valid_node_id(id) {
                    return Err(format!("invalid nodeId: {} — must use colon format e.g. 4029:12345", id));
                }
            }
            if let Some(format) = params.get("format").and_then(|v| v.as_str()) {
                if !valid_export_format(format) {
                    return Err(format!("format must be PNG, SVG, JPG, or PDF, got: {}", format));
                }
            }
        }

        "create_frame" => {
            if let Some(w) = params.get("width").and_then(|v| v.as_f64()) {
                if w <= 0.0 { return Err("width must be positive".into()); }
            }
            if let Some(h) = params.get("height").and_then(|v| v.as_f64()) {
                if h <= 0.0 { return Err("height must be positive".into()); }
            }
            if let Some(pid) = params.get("parentId").and_then(|v| v.as_str()) {
                if !pid.is_empty() && !valid_node_id(pid) {
                    return Err(format!("parentId must use colon format e.g. 4029:12345, got: {}", pid));
                }
            }
            validate_auto_layout_params(params)?;
        }

        "set_auto_layout" => {
            require_single_node_id(node_ids)?;
            validate_auto_layout_params(params)?;
        }

        "create_rectangle" | "create_ellipse" | "create_line" | "create_star" | "create_polygon" => {
            if let Some(w) = params.get("width").and_then(|v| v.as_f64()) {
                if w <= 0.0 { return Err("width must be positive".into()); }
            }
            if let Some(h) = params.get("height").and_then(|v| v.as_f64()) {
                if h <= 0.0 { return Err("height must be positive".into()); }
            }
            if let Some(pid) = params.get("parentId").and_then(|v| v.as_str()) {
                if !pid.is_empty() && !valid_node_id(pid) {
                    return Err(format!("parentId must use colon format e.g. 4029:12345, got: {}", pid));
                }
            }
        }

        "create_text" => {
            let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                return Err("text is required".into());
            }
            if let Some(pid) = params.get("parentId").and_then(|v| v.as_str()) {
                if !pid.is_empty() && !valid_node_id(pid) {
                    return Err(format!("parentId must use colon format e.g. 4029:12345, got: {}", pid));
                }
            }
        }

        "set_text" => {
            require_single_node_id(node_ids)?;
            if params.get("text").and_then(|v| v.as_str()).is_none() {
                return Err("text is required".into());
            }
        }

        "set_text_properties" => {
            require_single_node_id(node_ids)?;
        }

        "set_fills" | "set_strokes" => {
            validate_node_ids(node_ids)?;
            let color = params.get("color").and_then(|v| v.as_str()).unwrap_or("");
            if color.is_empty() {
                return Err("color is required (hex string e.g. #FF5733)".into());
            }
            if let Some(mode) = params.get("mode").and_then(|v| v.as_str()) {
                if mode != "replace" && mode != "append" {
                    return Err("mode must be 'replace' or 'append'".into());
                }
            }
        }

        "move_nodes" => {
            validate_node_ids(node_ids)?;
            if !params.contains_key("x") && !params.contains_key("y") {
                return Err("at least one of x or y is required".into());
            }
        }

        "resize_nodes" => {
            validate_node_ids(node_ids)?;
            if !params.contains_key("width") && !params.contains_key("height") {
                return Err("at least one of width or height is required".into());
            }
        }

        "delete_nodes" => {
            if node_ids.is_empty() {
                return Err("nodeIds is required and must not be empty".into());
            }
            validate_node_ids(node_ids)?;
        }

        "rename_node" => {
            require_single_node_id(node_ids)?;
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                return Err("name is required".into());
            }
        }

        "clone_node" => {
            require_single_node_id(node_ids)?;
            if let Some(pid) = params.get("parentId").and_then(|v| v.as_str()) {
                if !pid.is_empty() && !valid_node_id(pid) {
                    return Err(format!("parentId must use colon format e.g. 4029:12345, got: {}", pid));
                }
            }
        }

        "import_image" => {
            let image_data = params.get("imageData").and_then(|v| v.as_str()).unwrap_or("");
            if image_data.is_empty() {
                return Err("imageData (base64) is required".into());
            }
            if let Some(sm) = params.get("scaleMode").and_then(|v| v.as_str()) {
                if !sm.is_empty() && !["FILL", "FIT", "CROP", "TILE"].contains(&sm) {
                    return Err(format!("scaleMode must be FILL, FIT, CROP, or TILE, got: {}", sm));
                }
            }
            if let Some(pid) = params.get("parentId").and_then(|v| v.as_str()) {
                if !pid.is_empty() && !valid_node_id(pid) {
                    return Err(format!("parentId must use colon format e.g. 4029:12345, got: {}", pid));
                }
            }
        }

        // ── Style tools ──

        "create_paint_style" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() { return Err("name is required".into()); }
            let color = params.get("color").and_then(|v| v.as_str()).unwrap_or("");
            if color.is_empty() { return Err("color is required (hex string e.g. #FF5733)".into()); }
        }

        "create_text_style" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() { return Err("name is required".into()); }
            if let Some(td) = params.get("textDecoration").and_then(|v| v.as_str()) {
                if !td.is_empty() && !["NONE", "UNDERLINE", "STRIKETHROUGH"].contains(&td) {
                    return Err(format!("textDecoration must be NONE, UNDERLINE, or STRIKETHROUGH, got: {}", td));
                }
            }
            if let Some(unit) = params.get("lineHeightUnit").and_then(|v| v.as_str()) {
                if !unit.is_empty() && !["PIXELS", "PERCENT"].contains(&unit) {
                    return Err(format!("lineHeightUnit must be PIXELS or PERCENT, got: {}", unit));
                }
            }
            if let Some(unit) = params.get("letterSpacingUnit").and_then(|v| v.as_str()) {
                if !unit.is_empty() && !["PIXELS", "PERCENT"].contains(&unit) {
                    return Err(format!("letterSpacingUnit must be PIXELS or PERCENT, got: {}", unit));
                }
            }
        }

        "create_effect_style" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() { return Err("name is required".into()); }
            if let Some(t) = params.get("type").and_then(|v| v.as_str()) {
                if !t.is_empty() && !["DROP_SHADOW", "INNER_SHADOW", "LAYER_BLUR", "BACKGROUND_BLUR"].contains(&t) {
                    return Err(format!("type must be DROP_SHADOW, INNER_SHADOW, LAYER_BLUR, or BACKGROUND_BLUR, got: {}", t));
                }
            }
        }

        "create_grid_style" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() { return Err("name is required".into()); }
            if let Some(p) = params.get("pattern").and_then(|v| v.as_str()) {
                if !p.is_empty() && !["GRID", "COLUMNS", "ROWS"].contains(&p) {
                    return Err(format!("pattern must be GRID, COLUMNS, or ROWS, got: {}", p));
                }
            }
            if let Some(a) = params.get("alignment").and_then(|v| v.as_str()) {
                if !a.is_empty() && !["STRETCH", "CENTER", "MIN", "MAX"].contains(&a) {
                    return Err(format!("alignment must be STRETCH, CENTER, MIN, or MAX, got: {}", a));
                }
            }
        }

        "update_paint_style" | "update_text_style" | "update_effect_style" | "update_grid_style" => {
            let style_id = params.get("styleId").and_then(|v| v.as_str()).unwrap_or("");
            if style_id.is_empty() { return Err("styleId is required".into()); }
            if !params.contains_key("name") && !params.contains_key("description")
                && !params.contains_key("color") && !params.contains_key("fontSize")
                && !params.contains_key("fontFamily") && !params.contains_key("fontStyle")
                && !params.contains_key("type") && !params.contains_key("pattern")
                && !params.contains_key("radius") && !params.contains_key("count")
            {
                return Err("at least one updateable property is required".into());
            }
        }

        "delete_style" => {
            let style_id = params.get("styleId").and_then(|v| v.as_str()).unwrap_or("");
            if style_id.is_empty() { return Err("styleId is required".into()); }
        }

        // ── Variable tools ──

        "create_variable_collection" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() { return Err("name is required".into()); }
        }

        "add_variable_mode" => {
            let collection_id = params.get("collectionId").and_then(|v| v.as_str()).unwrap_or("");
            if collection_id.is_empty() { return Err("collectionId is required".into()); }
            let mode_name = params.get("modeName").and_then(|v| v.as_str()).unwrap_or("");
            if mode_name.is_empty() { return Err("modeName is required".into()); }
        }

        "create_variable" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() { return Err("name is required".into()); }
            let collection_id = params.get("collectionId").and_then(|v| v.as_str()).unwrap_or("");
            if collection_id.is_empty() { return Err("collectionId is required".into()); }
            let var_type = params.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !["COLOR", "FLOAT", "STRING", "BOOLEAN"].contains(&var_type) {
                return Err(format!("type must be COLOR, FLOAT, STRING, or BOOLEAN, got: {}", var_type));
            }
        }

        "set_variable_value" => {
            let variable_id = params.get("variableId").and_then(|v| v.as_str()).unwrap_or("");
            if variable_id.is_empty() { return Err("variableId is required".into()); }
            let mode_id = params.get("modeId").and_then(|v| v.as_str()).unwrap_or("");
            if mode_id.is_empty() { return Err("modeId is required".into()); }
            if !params.contains_key("value") { return Err("value is required".into()); }
        }

        "delete_variable" => {
            let vid = params.get("variableId").and_then(|v| v.as_str()).unwrap_or("");
            let cid = params.get("collectionId").and_then(|v| v.as_str()).unwrap_or("");
            if vid.is_empty() && cid.is_empty() {
                return Err("variableId or collectionId is required".into());
            }
        }

        // ── Linked tools ──

        "apply_style_to_node" => {
            require_single_node_id(node_ids)?;
            let style_id = params.get("styleId").and_then(|v| v.as_str()).unwrap_or("");
            if style_id.is_empty() { return Err("styleId is required".into()); }
            if let Some(target) = params.get("target").and_then(|v| v.as_str()) {
                if !target.is_empty() && !["fill", "stroke"].contains(&target) {
                    return Err(format!("target must be fill or stroke, got: {}", target));
                }
            }
        }

        "bind_variable_to_node" => {
            require_single_node_id(node_ids)?;
            let variable_id = params.get("variableId").and_then(|v| v.as_str()).unwrap_or("");
            if variable_id.is_empty() { return Err("variableId is required".into()); }
            let field = params.get("field").and_then(|v| v.as_str()).unwrap_or("");
            if field.is_empty() { return Err("field is required".into()); }
        }

        "swap_component" => {
            require_single_node_id(node_ids)?;
            let component_id = params.get("componentId").and_then(|v| v.as_str()).unwrap_or("");
            if component_id.is_empty() { return Err("componentId is required".into()); }
            if !valid_node_id(component_id) {
                return Err(format!("componentId must use colon format e.g. 4029:12345, got: {}", component_id));
            }
        }

        "detach_instance" => {
            if node_ids.is_empty() { return Err("nodeIds is required and must not be empty".into()); }
            validate_node_ids(node_ids)?;
        }

        // ── Prototype tools ──

        "set_reactions" => {
            require_single_node_id(node_ids)?;
            let reactions = params.get("reactions").ok_or("reactions is required")?;
            let reaction_list = reactions.as_array().ok_or("reactions must be an array")?;
            if let Some(mode) = params.get("mode").and_then(|v| v.as_str()) {
                if !mode.is_empty() && mode != "replace" && mode != "append" {
                    return Err(format!("mode must be 'replace' or 'append', got: {}", mode));
                }
            }
            for (i, raw) in reaction_list.iter().enumerate() {
                let r = raw.as_object().ok_or_else(|| format!("reactions[{}] must be an object", i))?;
                validate_reaction(i, r)?;
            }
        }

        "remove_reactions" => {
            require_single_node_id(node_ids)?;
            if let Some(raw) = params.get("indices").and_then(|v| v.as_array()) {
                for (i, v) in raw.iter().enumerate() {
                    if v.as_f64().is_none() {
                        return Err(format!("indices[{}] must be a number", i));
                    }
                }
            }
        }

        // ── Node Control ──

        "set_visible" => {
            validate_node_ids(node_ids)?;
            if params.get("visible").and_then(|v| v.as_bool()).is_none() {
                return Err("visible (boolean) is required".into());
            }
        }

        "set_locked" => {
            validate_node_ids(node_ids)?;
            if params.get("locked").and_then(|v| v.as_bool()).is_none() {
                return Err("locked (boolean) is required".into());
            }
        }

        "rotate_nodes" => {
            validate_node_ids(node_ids)?;
            if params.get("rotation").and_then(|v| v.as_f64()).is_none() {
                return Err("rotation (degrees) is required".into());
            }
        }

        "reorder_nodes" => {
            validate_node_ids(node_ids)?;
            let order = params.get("order").and_then(|v| v.as_str()).unwrap_or("");
            if !["bringToFront", "sendToBack", "bringForward", "sendBackward"].contains(&order) {
                return Err(format!("order must be bringToFront, sendToBack, bringForward, or sendBackward, got: {}", order));
            }
        }

        "set_blend_mode" => {
            validate_node_ids(node_ids)?;
            let blend_mode = params.get("blendMode").and_then(|v| v.as_str()).unwrap_or("");
            if blend_mode.is_empty() { return Err("blendMode is required".into()); }
            let valid_blend_modes = [
                "NORMAL","MULTIPLY","SCREEN","OVERLAY","DARKEN","LIGHTEN","COLOR_DODGE",
                "COLOR_BURN","HARD_LIGHT","SOFT_LIGHT","DIFFERENCE","EXCLUSION","HUE",
                "SATURATION","COLOR","LUMINOSITY","PASS_THROUGH"
            ];
            if !valid_blend_modes.contains(&blend_mode) {
                return Err(format!("blendMode {:?} is not a valid Figma blend mode", blend_mode));
            }
        }

        "set_constraints" => {
            validate_node_ids(node_ids)?;
            if !params.contains_key("horizontal") && !params.contains_key("vertical") {
                return Err("at least one of horizontal or vertical is required".into());
            }
            let valid_constraints = ["MIN", "MAX", "CENTER", "STRETCH", "SCALE"];
            if let Some(h) = params.get("horizontal").and_then(|v| v.as_str()) {
                if !h.is_empty() && !valid_constraints.contains(&h) {
                    return Err(format!("horizontal must be MIN, MAX, CENTER, STRETCH, or SCALE, got: {}", h));
                }
            }
            if let Some(v) = params.get("vertical").and_then(|v| v.as_str()) {
                if !v.is_empty() && !valid_constraints.contains(&v) {
                    return Err(format!("vertical must be MIN, MAX, CENTER, STRETCH, or SCALE, got: {}", v));
                }
            }
        }

        "reparent_nodes" => {
            validate_node_ids(node_ids)?;
            let parent_id = params.get("parentId").and_then(|v| v.as_str()).unwrap_or("");
            if parent_id.is_empty() { return Err("parentId is required".into()); }
            if !valid_node_id(parent_id) {
                return Err(format!("parentId must use colon format e.g. 4029:12345, got: {}", parent_id));
            }
        }

        "batch_rename_nodes" => {
            validate_node_ids(node_ids)?;
            let has_find = params.contains_key("find");
            let has_replace = params.contains_key("replace");
            let has_prefix = params.contains_key("prefix");
            let has_suffix = params.contains_key("suffix");
            if !has_find && !has_replace && !has_prefix && !has_suffix {
                return Err("at least one of find/replace, prefix, or suffix is required".into());
            }
            if has_find && !has_replace {
                return Err("replace is required when find is provided".into());
            }
        }

        "find_replace_text" => {
            let find = params.get("find").and_then(|v| v.as_str()).unwrap_or("");
            if find.is_empty() { return Err("find is required".into()); }
            if !params.contains_key("replace") { return Err("replace is required".into()); }
            if let Some(node_id) = params.get("nodeId").and_then(|v| v.as_str()) {
                if !node_id.is_empty() && !valid_node_id(node_id) {
                    return Err(format!("nodeId must use colon format e.g. 4029:12345, got: {}", node_id));
                }
            }
            if !node_ids.is_empty() && !node_ids[0].is_empty() && !valid_node_id(&node_ids[0]) {
                return Err(format!("nodeId must use colon format e.g. 4029:12345, got: {}", node_ids[0]));
            }
        }

        // ── Page management ──

        "add_page" => {
            if let Some(idx) = params.get("index").and_then(|v| v.as_f64()) {
                if idx < 0.0 { return Err("index must be non-negative".into()); }
            }
        }

        "delete_page" | "rename_page" => {
            let page_id = params.get("pageId").and_then(|v| v.as_str()).unwrap_or("");
            let page_name = params.get("pageName").and_then(|v| v.as_str()).unwrap_or("");
            if page_id.is_empty() && page_name.is_empty() {
                return Err("pageId or pageName is required".into());
            }
            if tool == "rename_page" {
                let new_name = params.get("newName").and_then(|v| v.as_str()).unwrap_or("");
                if new_name.is_empty() { return Err("newName is required".into()); }
            }
        }

        "set_effects" => {
            validate_node_ids(node_ids)?;
            let effects = params.get("effects").ok_or("effects array is required")?;
            let effect_list = effects.as_array().ok_or("effects must be an array")?;
            for (i, e) in effect_list.iter().enumerate() {
                let em = e.as_object().ok_or_else(|| format!("effects[{}] must be an object", i))?;
                let t = em.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if !["DROP_SHADOW", "INNER_SHADOW", "LAYER_BLUR", "BACKGROUND_BLUR"].contains(&t) {
                    return Err(format!("effects[{}].type must be DROP_SHADOW, INNER_SHADOW, LAYER_BLUR, or BACKGROUND_BLUR, got: {}", i, t));
                }
            }
        }

        "create_section" => {
            if let Some(w) = params.get("width").and_then(|v| v.as_f64()) {
                if w <= 0.0 { return Err("width must be positive".into()); }
            }
            if let Some(h) = params.get("height").and_then(|v| v.as_f64()) {
                if h <= 0.0 { return Err("height must be positive".into()); }
            }
        }

        "batch_create_nodes" => {
            let nodes = params.get("nodes").and_then(|v| v.as_array()).ok_or("nodes array is required")?;
            if nodes.is_empty() { return Err("nodes array must not be empty".into()); }
            for (i, n) in nodes.iter().enumerate() {
                let spec = n.as_object().ok_or_else(|| format!("nodes[{}] must be an object", i))?;
                let t = spec.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if !["frame", "rectangle", "ellipse", "text"].contains(&t) {
                    return Err(format!("nodes[{}].type must be frame, rectangle, ellipse, or text, got: {}", i, t));
                }
            }
        }

        "set_gradient_fill" => {
            validate_node_ids(node_ids)?;
            let gt = params.get("gradientType").and_then(|v| v.as_str()).unwrap_or("");
            if !["GRADIENT_LINEAR", "GRADIENT_RADIAL", "GRADIENT_DIAMOND", "GRADIENT_ANGULAR"].contains(&gt) {
                return Err(format!("gradientType must be GRADIENT_LINEAR, GRADIENT_RADIAL, GRADIENT_DIAMOND, or GRADIENT_ANGULAR, got: {}", gt));
            }
            let stops = params.get("stops").and_then(|v| v.as_array()).ok_or("stops array is required")?;
            if stops.len() < 2 { return Err("at least 2 gradient stops are required".into()); }
        }

        "set_viewport" => {
            // No strict validation — at least one of zoom/center/scrollTo should be provided
        }

        "set_plugin_data" => {
            let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("");
            if key.is_empty() { return Err("key is required".into()); }
            if !params.contains_key("value") { return Err("value is required".into()); }
        }

        "set_text_range" => {
            require_single_node_id(node_ids)?;
            if let Some(start) = params.get("start").and_then(|v| v.as_f64()) {
                if start < 0.0 { return Err("start must be non-negative".into()); }
            }
        }

        "set_component_property" => {
            require_single_node_id(node_ids)?;
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() { return Err("name is required".into()); }
            let pt = params.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if !["BOOLEAN", "TEXT", "VARIANT"].contains(&pt) {
                return Err(format!("type must be BOOLEAN, TEXT, or VARIANT, got: {}", pt));
            }
        }

        _ => {}
    }

    Ok(())
}

fn validate_node_ids(node_ids: &[String]) -> Result<(), String> {
    if node_ids.is_empty() {
        return Err("nodeIds is required".into());
    }
    for id in node_ids {
        if !valid_node_id(id) {
            return Err(format!("invalid nodeId: {} — must use colon format e.g. 4029:12345", id));
        }
    }
    Ok(())
}

/// Validate that node_ids contains exactly one non-empty, valid Figma node ID.
fn require_single_node_id(node_ids: &[String]) -> Result<(), String> {
    if node_ids.is_empty() || node_ids[0].is_empty() {
        return Err("nodeId is required".into());
    }
    if !valid_node_id(&node_ids[0]) {
        return Err(format!("nodeId must use colon format e.g. 4029:12345, got: {}", node_ids[0]));
    }
    Ok(())
}

static VALID_TRIGGER_TYPES: [&str; 9] = [
    "ON_CLICK", "ON_HOVER", "ON_PRESS", "ON_DRAG", "AFTER_TIMEOUT",
    "MOUSE_ENTER", "MOUSE_LEAVE", "MOUSE_UP", "MOUSE_DOWN",
];

static VALID_ACTION_TYPES: [&str; 8] = [
    "NODE", "BACK", "CLOSE", "URL",
    "CONDITIONAL", "SET_VARIABLE", "SET_VARIABLE_MODE", "UPDATE_MEDIA_RUNTIME",
];

fn validate_reaction(idx: usize, r: &Map<String, Value>) -> Result<(), String> {
    if let Some(trigger) = r.get("trigger").and_then(|v| v.as_object()) {
        validate_trigger_type(idx, trigger)?;
    }
    if let Some(action) = r.get("action").and_then(|v| v.as_object()) {
        validate_action_type(idx, action)?;
    }
    Ok(())
}

fn validate_trigger_type(idx: usize, trigger: &Map<String, Value>) -> Result<(), String> {
    let t = trigger.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if !t.is_empty() && !VALID_TRIGGER_TYPES.contains(&t) {
        return Err(format!("reactions[{}].trigger.type is invalid: {}", idx, t));
    }
    if t == "AFTER_TIMEOUT"
        && trigger.get("timeout").and_then(|v| v.as_f64()).is_none()
    {
        return Err(format!("reactions[{}].trigger.timeout is required for AFTER_TIMEOUT and must be a number (milliseconds)", idx));
    }
    Ok(())
}

fn validate_action_type(idx: usize, action: &Map<String, Value>) -> Result<(), String> {
    let t = action.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if !t.is_empty() && !VALID_ACTION_TYPES.contains(&t) {
        return Err(format!("reactions[{}].action.type is invalid: {}", idx, t));
    }
    match t {
        "NODE" => {
            let nav = action.get("navigation").and_then(|v| v.as_str()).unwrap_or("");
            if nav.is_empty() {
                return Err(format!("reactions[{}].action.navigation is required for NODE (e.g. NAVIGATE, OVERLAY, SCROLL_TO, SWAP, CHANGE_TO)", idx));
            }
        }
        "URL" => {
            let url = action.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if url.is_empty() {
                return Err(format!("reactions[{}].action.url is required for URL", idx));
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_auto_layout_params(params: &Map<String, Value>) -> Result<(), String> {
    if let Some(lm) = params.get("layoutMode").and_then(|v| v.as_str()) {
        if !lm.is_empty() && !["HORIZONTAL", "VERTICAL", "NONE"].contains(&lm) {
            return Err(format!("layoutMode must be HORIZONTAL, VERTICAL, or NONE, got: {}", lm));
        }
    }
    if let Some(v) = params.get("primaryAxisAlignItems").and_then(|v| v.as_str()) {
        if !v.is_empty() && !["MIN", "CENTER", "MAX", "SPACE_BETWEEN"].contains(&v) {
            return Err(format!("primaryAxisAlignItems must be MIN, CENTER, MAX, or SPACE_BETWEEN, got: {}", v));
        }
    }
    if let Some(v) = params.get("counterAxisAlignItems").and_then(|v| v.as_str()) {
        if !v.is_empty() && !["MIN", "CENTER", "MAX", "BASELINE"].contains(&v) {
            return Err(format!("counterAxisAlignItems must be MIN, CENTER, MAX, or BASELINE, got: {}", v));
        }
    }
    if let Some(v) = params.get("primaryAxisSizingMode").and_then(|v| v.as_str()) {
        if !v.is_empty() && !["FIXED", "AUTO"].contains(&v) {
            return Err(format!("primaryAxisSizingMode must be FIXED or AUTO, got: {}", v));
        }
    }
    if let Some(v) = params.get("counterAxisSizingMode").and_then(|v| v.as_str()) {
        if !v.is_empty() && !["FIXED", "AUTO"].contains(&v) {
            return Err(format!("counterAxisSizingMode must be FIXED or AUTO, got: {}", v));
        }
    }
    if let Some(v) = params.get("layoutWrap").and_then(|v| v.as_str()) {
        if !v.is_empty() && !["NO_WRAP", "WRAP"].contains(&v) {
            return Err(format!("layoutWrap must be NO_WRAP or WRAP, got: {}", v));
        }
    }
    Ok(())
}

fn valid_export_format(f: &str) -> bool {
    matches!(f, "PNG" | "SVG" | "JPG" | "PDF")
}
