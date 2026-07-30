use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult,
    Implementation, ListPromptsResult, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use serde_json::Value;

use base64::Engine;

use crate::node::Node;
use crate::prompts;
use crate::tools::*;
use crate::tools_read_defs::*;
use crate::tools_write_create_defs::*;
use crate::tools_write_component_defs::*;
use crate::tools_write_export_defs::*;
use crate::tools_write_modify_defs::*;
use crate::tools_write_page_defs::*;
use crate::tools_write_prototype_defs::*;
use crate::tools_write_style_defs::*;
use crate::tools_write_variable_defs::*;

/// The MCP server handler that routes tool calls to the Node.
pub struct FigmaMcpServer {
    node: Arc<Node>,
    tools: Vec<Tool>,
}

impl FigmaMcpServer {
    pub fn new(node: Arc<Node>) -> Self {
        let mut tools = Vec::new();
        tools.extend(read_tools());
        tools.extend(write_create_tools());
        tools.extend(write_modify_tools());
        tools.extend(write_style_tools());
        tools.extend(write_variable_tools());
        tools.extend(write_component_tools());
        tools.extend(write_prototype_tools());
        tools.extend(write_page_tools());
        tools.extend(write_export_tools());

        Self { node, tools }
    }

    async fn dispatch_tool(&self, params: &CallToolRequestParams) -> CallToolResult {
        let name = params.name.as_ref();
        let args = extract_arguments(params);

        // Special handling for get_screenshot with outputPath
        if name == "get_screenshot" {
            let output_path = get_str(&args, "outputPath");
            if !output_path.is_empty() {
                return self.handle_get_screenshot_to_file(&args).await;
            }
        }

        // Special handling for export_frames_to_pdf
        if name == "export_frames_to_pdf" {
            return self.handle_export_frames_to_pdf(&args).await;
        }

        let (node_ids, params_map) = build_rpc_params(name, &args);

        let result = self
            .node
            .send(name, node_ids, &mut Some(params_map))
            .await;

        render_response(result)
    }

    async fn handle_get_screenshot_to_file(&self, args: &serde_json::Map<String, Value>) -> CallToolResult {
        use crate::schema::normalize_node_id;

        let output_path = get_str(args, "outputPath");
        let format = get_str(args, "format");
        let scale = get_f64(args, "scale").unwrap_or(0.0);
        let node_ids: Vec<String> = get_str_array(args, "nodeIds")
            .into_iter()
            .map(|s| normalize_node_id(&s))
            .collect();

        let work_dir = match std::env::current_dir() {
            Ok(d) => d.to_string_lossy().to_string(),
            Err(e) => return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("getwd: {}", e))]),
        };

        let resolved_path = match resolve_output_path(&output_path, &work_dir) {
            Ok(p) => p,
            Err(e) => return CallToolResult::error(vec![rmcp::model::ContentBlock::text(e)]),
        };

        let mut fmt = if format.is_empty() {
            infer_format(&resolved_path).to_string()
        } else {
            format.to_string()
        };
        if fmt.is_empty() { fmt = "PNG".to_string(); }

        let mut params = serde_json::Map::new();
        params.insert("format".to_string(), Value::String(fmt.clone()));
        if scale > 0.0 { params.insert("scale".to_string(), serde_json::json!(scale)); }

        let ids = if node_ids.is_empty() { vec![] } else { node_ids.clone() };
        let result = self.node.send("get_screenshot", ids, &mut Some(params)).await;

        match result {
            Err(e) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(e)]),
            Ok(resp) if !resp.error.is_empty() => CallToolResult::error(vec![rmcp::model::ContentBlock::text(resp.error)]),
            Ok(resp) => {
                let wrapper = resp.data.unwrap_or(Value::Null);
                let exports = wrapper.get("exports").and_then(|v| v.as_array()).cloned().unwrap_or_default();

                if exports.is_empty() {
                    return CallToolResult::error(vec![rmcp::model::ContentBlock::text("no nodes to export")]);
                }

                let export = &exports[0];
                let base64_data = export.get("base64").and_then(|v| v.as_str()).unwrap_or("");

                match write_base64(base64_data, &resolved_path) {
                    Ok(bytes) => {
                        let out = serde_json::json!({
                            "nodeId": export.get("nodeId").and_then(|v| v.as_str()).unwrap_or(""),
                            "nodeName": export.get("nodeName").and_then(|v| v.as_str()).unwrap_or(""),
                            "outputPath": resolved_path,
                            "format": fmt,
                            "width": export.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "height": export.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            "bytesWritten": bytes,
                            "success": true
                        });
                        CallToolResult::success(vec![rmcp::model::ContentBlock::text(
                            serde_json::to_string(&out).unwrap_or_default(),
                        )])
                    }
                    Err(e) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(e)]),
                }
            }
        }
    }

    async fn handle_export_frames_to_pdf(&self, args: &serde_json::Map<String, Value>) -> CallToolResult {
        use crate::pdf::merge_pdf_pages;
        use crate::schema::normalize_node_id;

        let raw_node_ids = args.get("nodeIds").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let node_ids: Vec<String> = raw_node_ids.iter()
            .filter_map(|v| v.as_str().map(normalize_node_id))
            .collect();
        let output_path = args.get("outputPath").and_then(|v| v.as_str()).unwrap_or("");

        if output_path.is_empty() {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text("outputPath is required")]);
        }

        let work_dir = match std::env::current_dir() {
            Ok(d) => d.to_string_lossy().to_string(),
            Err(e) => return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("getwd: {}", e))]),
        };

        let resolved_path = match resolve_output_path(output_path, &work_dir) {
            Ok(p) => p,
            Err(e) => return CallToolResult::error(vec![rmcp::model::ContentBlock::text(e)]),
        };

        let ext = std::path::Path::new(&resolved_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext.to_lowercase() != "pdf" {
            return CallToolResult::error(vec![rmcp::model::ContentBlock::text("outputPath must have a .pdf extension")]);
        }

        let result = self.node.send("export_frames_to_pdf", node_ids, &mut None).await;
        match result {
            Err(e) => CallToolResult::error(vec![rmcp::model::ContentBlock::text(e)]),
            Ok(resp) if !resp.error.is_empty() => CallToolResult::error(vec![rmcp::model::ContentBlock::text(resp.error)]),
            Ok(resp) => {
                let data = resp.data.unwrap_or(Value::Null);
                let frames = data.get("frames").and_then(|v| v.as_array()).cloned().unwrap_or_default();

                if frames.is_empty() {
                    return CallToolResult::error(vec![rmcp::model::ContentBlock::text("no PDF frames returned by plugin")]);
                }

                let mut pages = Vec::new();
                for (i, f) in frames.iter().enumerate() {
                    let b64 = f.get("base64").and_then(|v| v.as_str()).unwrap_or("");
                    if b64.is_empty() {
                        return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("frame {} has empty base64", i))]);
                    }
                    match base64::engine::general_purpose::STANDARD.decode(b64) {
                        Ok(raw) => pages.push(raw),
                        Err(e) => return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("frame {}: base64 decode: {}", i, e))]),
                    }
                }

                let merged = match merge_pdf_pages(&pages) {
                    Ok(m) => m,
                    Err(e) => return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("merge PDFs: {}", e))]),
                };

                if let Some(parent) = std::path::Path::new(&resolved_path).parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("mkdir: {}", e))]);
                    }
                }

                if std::path::Path::new(&resolved_path).exists() {
                    return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("file already exists: {}", resolved_path))]);
                }

                if let Err(e) = std::fs::write(&resolved_path, &merged) {
                    return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("write file: {}", e))]);
                }

                let out = serde_json::json!({
                    "outputPath": resolved_path,
                    "bytesWritten": merged.len(),
                    "pageCount": pages.len(),
                    "success": true
                });
                CallToolResult::success(vec![rmcp::model::ContentBlock::text(
                    serde_json::to_string(&out).unwrap_or_default(),
                )])
            }
        }
    }
}

impl ServerHandler for FigmaMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new("figma-mcp", env!("CARGO_PKG_VERSION")))
        .with_instructions("Figma MCP server with full read/write access via plugin bridge.")
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(self.tools.clone()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        Ok(self.dispatch_tool(&request).await)
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(prompts::all_prompts()))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        prompts::get_prompt_result(request.name.as_ref())
            .ok_or_else(|| McpError::invalid_params(format!("prompt not found: {}", request.name), None))
    }
}

/// Build (node_ids, params) from MCP arguments based on tool name.
/// Delegates to per-category builder functions; falls back to passthrough.
fn build_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> (Vec<String>, serde_json::Map<String, Value>) {
    build_read_rpc_params(tool, args)
        .or_else(|| build_create_rpc_params(tool, args))
        .or_else(|| build_modify_rpc_params(tool, args))
        .or_else(|| build_style_rpc_params(tool, args))
        .or_else(|| build_variable_rpc_params(tool, args))
        .or_else(|| build_prototype_rpc_params(tool, args))
        .or_else(|| build_page_rpc_params(tool, args))
        .unwrap_or_else(|| (vec![], args.clone()))
}

fn build_read_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> Option<(Vec<String>, serde_json::Map<String, Value>)> {
    use crate::schema::normalize_node_id;

    match tool {
        "get_pages" | "get_metadata" | "get_selection" | "get_viewport"
        | "get_fonts" | "get_styles" | "get_variable_defs" | "get_local_components" => {
            Some((vec![], serde_json::Map::new()))
        }

        "get_reactions" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            Some((vec![node_id], serde_json::Map::new()))
        }

        "get_nodes_info" | "export_frames_to_pdf" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            Some((ids, serde_json::Map::new()))
        }

        "get_design_context" => {
            let mut params = serde_json::Map::new();
            if let Some(d) = get_f64(args, "depth") {
                if d > 0.0 { params.insert("depth".into(), serde_json::json!(d)); }
            }
            let detail = get_str(args, "detail");
            if !detail.is_empty() { params.insert("detail".into(), Value::String(detail)); }
            if let Some(dd) = get_bool(args, "dedupe_components") {
                if dd { params.insert("dedupeComponents".into(), Value::Bool(true)); }
            }
            Some((vec![], params))
        }

        "search_nodes" => {
            let mut params = serde_json::Map::new();
            params.insert("query".into(), Value::String(get_str(args, "query")));
            let id = get_str(args, "nodeId");
            if !id.is_empty() { params.insert("nodeId".into(), Value::String(id)); }
            if let Some(raw) = args.get("types").and_then(|v| v.as_array()) {
                if !raw.is_empty() { params.insert("types".into(), Value::Array(raw.clone())); }
            }
            if let Some(limit) = get_f64(args, "limit") {
                if limit > 0.0 { params.insert("limit".into(), serde_json::json!(limit)); }
            }
            Some((vec![], params))
        }

        "scan_nodes_by_types" => {
            let mut params = serde_json::Map::new();
            params.insert("nodeId".into(), Value::String(get_str(args, "nodeId")));
            if let Some(raw) = args.get("types").cloned() {
                params.insert("types".into(), raw);
            }
            Some((vec![], params))
        }

        "get_screenshot" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            let f = get_str(args, "format");
            if !f.is_empty() { params.insert("format".into(), Value::String(f)); }
            if let Some(s) = get_f64(args, "scale") {
                if s > 0.0 { params.insert("scale".into(), serde_json::json!(s)); }
            }
            Some((ids, params))
        }

        "get_annotations" => {
            let mut params = serde_json::Map::new();
            let id = get_str(args, "nodeId");
            if !id.is_empty() { params.insert("nodeId".into(), Value::String(id)); }
            Some((vec![], params))
        }

        "export_tokens" => {
            let mut params = serde_json::Map::new();
            let f = get_str(args, "format");
            if !f.is_empty() { params.insert("format".into(), Value::String(f)); }
            Some((vec![], params))
        }

        _ => None,
    }
}

fn build_create_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> Option<(Vec<String>, serde_json::Map<String, Value>)> {
    use crate::schema::normalize_node_id;

    match tool {
        "create_frame" | "create_rectangle" | "create_ellipse" | "create_text"
        | "create_line" | "create_star" | "create_polygon" => {
            Some((vec![], args.clone()))
        }

        "import_image" => {
            let mut params = serde_json::Map::new();
            params.insert("imageData".into(), args.get("imageData").cloned().unwrap_or(Value::Null));
            copy_opt_f64(args, &mut params, "x");
            copy_opt_f64(args, &mut params, "y");
            copy_opt_f64(args, &mut params, "width");
            copy_opt_f64(args, &mut params, "height");
            copy_opt_str(args, &mut params, "name");
            copy_opt_str(args, &mut params, "scaleMode");
            copy_opt_str(args, &mut params, "parentId");
            Some((vec![], params))
        }

        "create_component" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            Some((vec![node_id], params))
        }

        "create_section" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            copy_opt_f64(args, &mut params, "x");
            copy_opt_f64(args, &mut params, "y");
            copy_opt_f64(args, &mut params, "width");
            copy_opt_f64(args, &mut params, "height");
            Some((vec![], params))
        }

        _ => None,
    }
}

fn build_modify_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> Option<(Vec<String>, serde_json::Map<String, Value>)> {
    use crate::schema::normalize_node_id;

    match tool {
        "set_text" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("text".into(), Value::String(get_str(args, "text")));
            Some((vec![node_id], params))
        }

        "set_text_properties" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            Some((vec![node_id], args.clone()))
        }

        "set_fills" | "set_strokes" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            params.insert("color".into(), args.get("color").cloned().unwrap_or(Value::Null));
            copy_opt_f64(args, &mut params, "opacity");
            copy_opt_f64(args, &mut params, "strokeWeight");
            copy_opt_str(args, &mut params, "mode");
            Some((ids, params))
        }

        "move_nodes" | "resize_nodes" | "delete_nodes" | "set_visible" | "set_locked"
        | "rotate_nodes" | "reorder_nodes" | "set_blend_mode" | "set_constraints"
        | "batch_rename_nodes" | "ungroup_nodes" | "detach_instance" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            match tool {
                "move_nodes" => { copy_opt_f64(args, &mut params, "x"); copy_opt_f64(args, &mut params, "y"); }
                "resize_nodes" => { copy_opt_f64(args, &mut params, "width"); copy_opt_f64(args, &mut params, "height"); }
                "set_visible" => { if let Some(v) = get_bool(args, "visible") { params.insert("visible".into(), Value::Bool(v)); } }
                "set_locked" => { if let Some(v) = get_bool(args, "locked") { params.insert("locked".into(), Value::Bool(v)); } }
                "rotate_nodes" => { if let Some(v) = get_f64(args, "rotation") { params.insert("rotation".into(), serde_json::json!(v)); } }
                "reorder_nodes" => { copy_opt_str(args, &mut params, "order"); }
                "set_blend_mode" => { copy_opt_str(args, &mut params, "blendMode"); }
                "set_constraints" => { copy_opt_str(args, &mut params, "horizontal"); copy_opt_str(args, &mut params, "vertical"); }
                "batch_rename_nodes" => {
                    for k in &["find", "replace", "regexFlags", "prefix", "suffix"] {
                        copy_opt_str(args, &mut params, k);
                    }
                    if let Some(v) = get_bool(args, "useRegex") { params.insert("useRegex".into(), Value::Bool(v)); }
                }
                _ => {}
            }
            Some((ids, params))
        }

        "group_nodes" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            Some((ids, params))
        }

        "rename_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("name".into(), Value::String(get_str(args, "name")));
            Some((vec![node_id], params))
        }

        "clone_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            copy_opt_f64(args, &mut params, "x");
            copy_opt_f64(args, &mut params, "y");
            copy_opt_str(args, &mut params, "parentId");
            Some((vec![node_id], params))
        }

        "set_opacity" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            if let Some(v) = get_f64(args, "opacity") { params.insert("opacity".into(), serde_json::json!(v)); }
            Some((ids, params))
        }

        "set_corner_radius" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            for k in &["cornerRadius", "topLeftRadius", "topRightRadius", "bottomLeftRadius", "bottomRightRadius"] {
                copy_opt_f64(args, &mut params, k);
            }
            Some((ids, params))
        }

        "set_auto_layout" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            Some((vec![node_id], args.clone()))
        }

        "reparent_nodes" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            let pid = normalize_node_id(&get_str(args, "parentId"));
            params.insert("parentId".into(), Value::String(pid));
            Some((ids, params))
        }

        "find_replace_text" => {
            let mut params = serde_json::Map::new();
            params.insert("find".into(), args.get("find").cloned().unwrap_or(Value::Null));
            params.insert("replace".into(), args.get("replace").cloned().unwrap_or(Value::Null));
            if let Some(v) = get_bool(args, "useRegex") { params.insert("useRegex".into(), Value::Bool(v)); }
            copy_opt_str(args, &mut params, "regexFlags");
            let mut node_ids = vec![];
            let node_id = get_str(args, "nodeId");
            if !node_id.is_empty() {
                node_ids.push(normalize_node_id(&node_id));
            }
            Some((node_ids, params))
        }

        "set_effects" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            params.insert("effects".into(), args.get("effects").cloned().unwrap_or(Value::Null));
            Some((ids, params))
        }

        "swap_component" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let component_id = normalize_node_id(&get_str(args, "componentId"));
            let mut params = serde_json::Map::new();
            params.insert("componentId".into(), Value::String(component_id));
            Some((vec![node_id], params))
        }

        _ => None,
    }
}

fn build_style_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> Option<(Vec<String>, serde_json::Map<String, Value>)> {
    use crate::schema::normalize_node_id;

    match tool {
        "create_paint_style" | "create_text_style" | "create_effect_style" | "create_grid_style"
        | "update_paint_style" | "update_text_style" | "update_effect_style" | "update_grid_style"
        | "delete_style" => {
            Some((vec![], args.clone()))
        }

        "apply_style_to_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("styleId".into(), args.get("styleId").cloned().unwrap_or(Value::Null));
            copy_opt_str(args, &mut params, "target");
            Some((vec![node_id], params))
        }

        _ => None,
    }
}

fn build_variable_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> Option<(Vec<String>, serde_json::Map<String, Value>)> {
    use crate::schema::normalize_node_id;

    match tool {
        "create_variable_collection" | "add_variable_mode" | "create_variable"
        | "set_variable_value" | "delete_variable" => {
            Some((vec![], args.clone()))
        }

        "bind_variable_to_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("variableId".into(), args.get("variableId").cloned().unwrap_or(Value::Null));
            params.insert("field".into(), args.get("field").cloned().unwrap_or(Value::Null));
            Some((vec![node_id], params))
        }

        _ => None,
    }
}

fn build_prototype_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> Option<(Vec<String>, serde_json::Map<String, Value>)> {
    use crate::schema::normalize_node_id;

    match tool {
        "set_reactions" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("reactions".into(), args.get("reactions").cloned().unwrap_or(Value::Null));
            copy_opt_str(args, &mut params, "mode");
            Some((vec![node_id], params))
        }

        "remove_reactions" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            if let Some(indices) = args.get("indices").cloned() {
                params.insert("indices".into(), indices);
            }
            Some((vec![node_id], params))
        }

        _ => None,
    }
}

fn build_page_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> Option<(Vec<String>, serde_json::Map<String, Value>)> {
    match tool {
        "navigate_to_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "pageId");
            copy_opt_str(args, &mut params, "pageName");
            Some((vec![], params))
        }

        "add_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            if let Some(idx) = get_f64(args, "index") { params.insert("index".into(), serde_json::json!(idx)); }
            Some((vec![], params))
        }

        "delete_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "pageId");
            copy_opt_str(args, &mut params, "pageName");
            Some((vec![], params))
        }

        "rename_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "pageId");
            copy_opt_str(args, &mut params, "pageName");
            copy_opt_str(args, &mut params, "newName");
            Some((vec![], params))
        }

        _ => None,
    }
}

fn copy_opt_str(args: &serde_json::Map<String, Value>, params: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(v) = args.get(key) {
        if v.as_str().is_none_or(|s| !s.is_empty()) {
            params.insert(key.to_string(), v.clone());
        }
    }
}

fn copy_opt_f64(args: &serde_json::Map<String, Value>, params: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(v) = args.get(key).and_then(|v| v.as_f64()) {
        params.insert(key.to_string(), serde_json::json!(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_args(pairs: &[(&str, Value)]) -> serde_json::Map<String, Value> {
        let mut m = serde_json::Map::new();
        for (k, v) in pairs {
            m.insert((*k).to_string(), v.clone());
        }
        m
    }

    // ── Read tools ──

    #[test]
    fn test_build_rpc_simple_no_params() {
        let args = serde_json::Map::new();
        for tool in &["get_pages", "get_metadata", "get_selection", "get_viewport",
                       "get_fonts", "get_styles", "get_variable_defs", "get_local_components"] {
            let (ids, params) = build_rpc_params(tool, &args);
            assert!(ids.is_empty(), "{} should have no node_ids", tool);
            assert!(params.is_empty(), "{} should have no params", tool);
        }
    }

    #[test]
    fn test_build_rpc_get_reactions() {
        let args = make_args(&[("nodeId", json!("4029:12345"))]);
        let (ids, params) = build_rpc_params("get_reactions", &args);
        assert_eq!(ids, vec!["4029:12345"]);
        assert!(params.is_empty());
    }

    #[test]
    fn test_build_rpc_get_nodes_info() {
        let args = make_args(&[("nodeIds", json!(["1:2", "3:4"]))]);
        let (ids, params) = build_rpc_params("get_nodes_info", &args);
        assert_eq!(ids, vec!["1:2", "3:4"]);
        assert!(params.is_empty());
    }

    #[test]
    fn test_build_rpc_get_nodes_info_hyphen_normalized() {
        let args = make_args(&[("nodeIds", json!(["1-2"]))]);
        let (ids, _) = build_rpc_params("get_nodes_info", &args);
        assert_eq!(ids, vec!["1:2"]);
    }

    #[test]
    fn test_build_rpc_get_design_context() {
        let args = make_args(&[
            ("depth", json!(3)),
            ("detail", json!("compact")),
            ("dedupe_components", json!(true)),
        ]);
        let (ids, params) = build_rpc_params("get_design_context", &args);
        assert!(ids.is_empty());
        assert_eq!(params.get("depth").unwrap(), &json!(3.0));
        assert_eq!(params.get("detail").unwrap(), &json!("compact"));
        assert_eq!(params.get("dedupeComponents").unwrap(), &json!(true));
    }

    #[test]
    fn test_build_rpc_search_nodes() {
        let args = make_args(&[("query", json!("button"))]);
        let (ids, params) = build_rpc_params("search_nodes", &args);
        assert!(ids.is_empty());
        assert_eq!(params.get("query").unwrap(), &json!("button"));
    }

    #[test]
    fn test_build_rpc_get_screenshot() {
        let args = make_args(&[
            ("nodeIds", json!(["10:20"])),
            ("format", json!("PNG")),
            ("scale", json!(2.0)),
        ]);
        let (ids, params) = build_rpc_params("get_screenshot", &args);
        assert_eq!(ids, vec!["10:20"]);
        assert_eq!(params.get("format").unwrap(), &json!("PNG"));
        assert_eq!(params.get("scale").unwrap(), &json!(2.0));
    }

    // ── Create tools ──

    #[test]
    fn test_build_rpc_create_frame_passthrough() {
        let args = make_args(&[("width", json!(100.0)), ("height", json!(200.0))]);
        let (ids, params) = build_rpc_params("create_frame", &args);
        assert!(ids.is_empty());
        assert_eq!(params.get("width").unwrap(), &json!(100.0));
        assert_eq!(params.get("height").unwrap(), &json!(200.0));
    }

    #[test]
    fn test_build_rpc_import_image() {
        let args = make_args(&[
            ("imageData", json!("base64data")),
            ("x", json!(10.0)),
            ("y", json!(20.0)),
            ("width", json!(100.0)),
            ("height", json!(100.0)),
            ("name", json!("test")),
            ("scaleMode", json!("FILL")),
            ("parentId", json!("5:6")),
        ]);
        let (ids, params) = build_rpc_params("import_image", &args);
        assert!(ids.is_empty());
        assert_eq!(params.get("imageData").unwrap(), &json!("base64data"));
        assert_eq!(params.get("x").unwrap(), &json!(10.0));
        assert_eq!(params.get("scaleMode").unwrap(), &json!("FILL"));
        assert_eq!(params.get("parentId").unwrap(), &json!("5:6"));
    }

    #[test]
    fn test_build_rpc_create_component() {
        let args = make_args(&[("nodeId", json!("7:8")), ("name", json!("Button"))]);
        let (ids, params) = build_rpc_params("create_component", &args);
        assert_eq!(ids, vec!["7:8"]);
        assert_eq!(params.get("name").unwrap(), &json!("Button"));
    }

    // ── Modify tools ──

    #[test]
    fn test_build_rpc_set_text() {
        let args = make_args(&[("nodeId", json!("1:2")), ("text", json!("hello"))]);
        let (ids, params) = build_rpc_params("set_text", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("text").unwrap(), &json!("hello"));
    }

    #[test]
    fn test_build_rpc_set_fills() {
        let args = make_args(&[
            ("nodeIds", json!(["1:2", "3:4"])),
            ("color", json!("#FF5733")),
            ("opacity", json!(0.5)),
        ]);
        let (ids, params) = build_rpc_params("set_fills", &args);
        assert_eq!(ids, vec!["1:2", "3:4"]);
        assert_eq!(params.get("color").unwrap(), &json!("#FF5733"));
        assert_eq!(params.get("opacity").unwrap(), &json!(0.5));
    }

    #[test]
    fn test_build_rpc_move_nodes() {
        let args = make_args(&[
            ("nodeIds", json!(["1:2"])),
            ("x", json!(100.0)),
            ("y", json!(200.0)),
        ]);
        let (ids, params) = build_rpc_params("move_nodes", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("x").unwrap(), &json!(100.0));
        assert_eq!(params.get("y").unwrap(), &json!(200.0));
    }

    #[test]
    fn test_build_rpc_rename_node() {
        let args = make_args(&[("nodeId", json!("1:2")), ("name", json!("NewName"))]);
        let (ids, params) = build_rpc_params("rename_node", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("name").unwrap(), &json!("NewName"));
    }

    #[test]
    fn test_build_rpc_clone_node() {
        let args = make_args(&[
            ("nodeId", json!("1:2")),
            ("x", json!(50.0)),
            ("parentId", json!("3:4")),
        ]);
        let (ids, params) = build_rpc_params("clone_node", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("x").unwrap(), &json!(50.0));
        assert_eq!(params.get("parentId").unwrap(), &json!("3:4"));
    }

    #[test]
    fn test_build_rpc_set_auto_layout_passthrough() {
        let args = make_args(&[("nodeId", json!("1:2")), ("layoutMode", json!("HORIZONTAL"))]);
        let (ids, params) = build_rpc_params("set_auto_layout", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("layoutMode").unwrap(), &json!("HORIZONTAL"));
    }

    #[test]
    fn test_build_rpc_swap_component() {
        let args = make_args(&[
            ("nodeId", json!("1:2")),
            ("componentId", json!("3:4")),
        ]);
        let (ids, params) = build_rpc_params("swap_component", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("componentId").unwrap(), &json!("3:4"));
    }

    // ── Style tools ──

    #[test]
    fn test_build_rpc_apply_style_to_node() {
        let args = make_args(&[
            ("nodeId", json!("1:2")),
            ("styleId", json!("S:123")),
            ("target", json!("fill")),
        ]);
        let (ids, params) = build_rpc_params("apply_style_to_node", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("styleId").unwrap(), &json!("S:123"));
        assert_eq!(params.get("target").unwrap(), &json!("fill"));
    }

    // ── Variable tools ──

    #[test]
    fn test_build_rpc_bind_variable_to_node() {
        let args = make_args(&[
            ("nodeId", json!("1:2")),
            ("variableId", json!("V:42")),
            ("field", json!("fillColor")),
        ]);
        let (ids, params) = build_rpc_params("bind_variable_to_node", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("variableId").unwrap(), &json!("V:42"));
        assert_eq!(params.get("field").unwrap(), &json!("fillColor"));
    }

    // ── Prototype tools ──

    #[test]
    fn test_build_rpc_set_reactions() {
        let reactions = json!([{"trigger": {"type": "ON_CLICK"}}]);
        let args = make_args(&[
            ("nodeId", json!("1:2")),
            ("reactions", reactions.clone()),
            ("mode", json!("replace")),
        ]);
        let (ids, params) = build_rpc_params("set_reactions", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("reactions").unwrap(), &reactions);
        assert_eq!(params.get("mode").unwrap(), &json!("replace"));
    }

    #[test]
    fn test_build_rpc_remove_reactions() {
        let args = make_args(&[("nodeId", json!("1:2")), ("indices", json!([0, 1]))]);
        let (ids, params) = build_rpc_params("remove_reactions", &args);
        assert_eq!(ids, vec!["1:2"]);
        assert_eq!(params.get("indices").unwrap(), &json!([0, 1]));
    }

    // ── Page tools ──

    #[test]
    fn test_build_rpc_add_page() {
        let args = make_args(&[("name", json!("My Page")), ("index", json!(2.0))]);
        let (ids, params) = build_rpc_params("add_page", &args);
        assert!(ids.is_empty());
        assert_eq!(params.get("name").unwrap(), &json!("My Page"));
        assert_eq!(params.get("index").unwrap(), &json!(2.0));
    }

    #[test]
    fn test_build_rpc_navigate_to_page() {
        let args = make_args(&[("pageId", json!("0:2"))]);
        let (ids, params) = build_rpc_params("navigate_to_page", &args);
        assert!(ids.is_empty());
        assert_eq!(params.get("pageId").unwrap(), &json!("0:2"));
    }

    // ── Fallback ──

    #[test]
    fn test_build_rpc_unknown_tool_passthrough() {
        let args = make_args(&[("foo", json!("bar"))]);
        let (ids, params) = build_rpc_params("unknown_tool", &args);
        assert!(ids.is_empty());
        assert_eq!(params.get("foo").unwrap(), &json!("bar"));
    }
}
