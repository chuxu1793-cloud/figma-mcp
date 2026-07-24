use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult,
    Implementation, ListPromptsResult, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler, ServiceExt};
use serde_json::Value;
use tracing::info;

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

        // Special handling for save_screenshots
        if name == "save_screenshots" {
            return self.handle_save_screenshots(&args).await;
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

        render_response(result.map_err(|e| e))
    }

    async fn handle_save_screenshots(&self, args: &serde_json::Map<String, Value>) -> CallToolResult {
        let raw_items = args.get("items").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let default_format = args.get("format").and_then(|v| v.as_str()).unwrap_or("");
        let default_scale = args.get("scale").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let work_dir = match std::env::current_dir() {
            Ok(d) => d.to_string_lossy().to_string(),
            Err(e) => {
                return CallToolResult::error(vec![rmcp::model::ContentBlock::text(format!("getwd: {}", e))]);
            }
        };

        let mut results = Vec::new();
        let mut succeeded = 0;
        let mut failed = 0;

        for (i, raw_item) in raw_items.iter().enumerate() {
            let item = match raw_item.as_object() {
                Some(m) => m,
                None => {
                    results.push(serde_json::json!({
                        "index": i,
                        "error": format!("items[{}] must be an object", i)
                    }));
                    failed += 1;
                    continue;
                }
            };

            let node_id = item.get("nodeId").and_then(|v| v.as_str()).unwrap_or("");
            let output_path = item.get("outputPath").and_then(|v| v.as_str()).unwrap_or("");
            let format = item.get("format").and_then(|v| v.as_str()).unwrap_or("");
            let scale = item.get("scale").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let r = self.save_screenshot_item(node_id, output_path, format, scale, i, &work_dir, default_format, default_scale).await;
            if r.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                succeeded += 1;
            } else {
                failed += 1;
            }
            results.push(r);
        }

        let out = serde_json::json!({
            "total": results.len(),
            "succeeded": succeeded,
            "failed": failed,
            "hasErrors": failed > 0,
            "results": results,
        });

        CallToolResult::success(vec![rmcp::model::ContentBlock::text(
            serde_json::to_string(&out).unwrap_or_default(),
        )])
    }

    async fn save_screenshot_item(
        &self,
        node_id: &str,
        output_path: &str,
        format: &str,
        scale: f64,
        index: usize,
        work_dir: &str,
        default_format: &str,
        default_scale: f64,
    ) -> Value {
        let resolved_path = match resolve_output_path(output_path, work_dir) {
            Ok(p) => p,
            Err(e) => return serde_json::json!({"index": index, "nodeId": node_id, "outputPath": output_path, "error": e}),
        };

        let mut fmt = coalesce(format, default_format).to_string();
        let inferred = infer_format(&resolved_path).to_string();
        if fmt.is_empty() {
            fmt = inferred.clone();
        }
        if fmt.is_empty() {
            fmt = "PNG".to_string();
        }
        if !inferred.is_empty() && fmt != inferred {
            return serde_json::json!({
                "index": index, "nodeId": node_id, "outputPath": resolved_path,
                "error": format!("format {} conflicts with file extension {}", fmt, inferred)
            });
        }

        let mut s = scale;
        if s <= 0.0 {
            s = default_scale;
        }

        let mut params = serde_json::Map::new();
        params.insert("format".to_string(), Value::String(fmt.clone()));
        if s > 0.0 {
            params.insert("scale".to_string(), serde_json::json!(s));
        }

        let result = self.node.send("get_screenshot", vec![node_id.to_string()], &mut Some(params)).await;
        match result {
            Err(e) => serde_json::json!({"index": index, "nodeId": node_id, "outputPath": resolved_path, "error": e}),
            Ok(resp) if !resp.error.is_empty() => serde_json::json!({"index": index, "nodeId": node_id, "outputPath": resolved_path, "error": resp.error}),
            Ok(resp) => {
                // Extract screenshot export from response data
                let wrapper = resp.data.unwrap_or(Value::Null);
                let exports = wrapper.get("exports").and_then(|v| v.as_array());
                let export = exports.and_then(|a| a.first()).cloned().unwrap_or(Value::Null);

                let base64_data = export.get("base64").and_then(|v| v.as_str()).unwrap_or("");
                let export_node_id = export.get("nodeId").and_then(|v| v.as_str()).unwrap_or(node_id);
                let export_node_name = export.get("nodeName").and_then(|v| v.as_str()).unwrap_or("");
                let export_width = export.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let export_height = export.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);

                match write_base64(base64_data, &resolved_path) {
                    Ok(bytes) => serde_json::json!({
                        "index": index,
                        "nodeId": export_node_id,
                        "nodeName": export_node_name,
                        "outputPath": resolved_path,
                        "format": fmt,
                        "width": export_width,
                        "height": export_height,
                        "bytesWritten": bytes,
                        "success": true
                    }),
                    Err(e) => serde_json::json!({
                        "index": index, "nodeId": node_id, "outputPath": resolved_path, "error": e
                    }),
                }
            }
        }
    }

    async fn handle_export_frames_to_pdf(&self, args: &serde_json::Map<String, Value>) -> CallToolResult {
        use crate::pdf::merge_pdf_pages;
        use crate::schema::normalize_node_id;

        let raw_node_ids = args.get("nodeIds").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let node_ids: Vec<String> = raw_node_ids.iter()
            .filter_map(|v| v.as_str().map(|s| normalize_node_id(s)))
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
fn build_rpc_params(tool: &str, args: &serde_json::Map<String, Value>) -> (Vec<String>, serde_json::Map<String, Value>) {
    use crate::schema::normalize_node_id;

    match tool {
        // Simple tools: no params, no node_ids
        "get_document" | "get_pages" | "get_metadata" | "get_selection" | "get_viewport"
        | "get_fonts" | "get_styles" | "get_variable_defs" | "get_local_components" => {
            (vec![], serde_json::Map::new())
        }

        // Tools with nodeId parameter
        "get_node" | "get_reactions" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            (vec![node_id], serde_json::Map::new())
        }

        // Tools with nodeIds parameter
        "get_nodes_info" | "export_frames_to_pdf" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            (ids, serde_json::Map::new())
        }

        // get_design_context
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
            (vec![], params)
        }

        // search_nodes
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
            (vec![], params)
        }

        // scan_text_nodes
        "scan_text_nodes" => {
            let mut params = serde_json::Map::new();
            params.insert("nodeId".into(), Value::String(get_str(args, "nodeId")));
            (vec![], params)
        }

        // scan_nodes_by_types
        "scan_nodes_by_types" => {
            let mut params = serde_json::Map::new();
            params.insert("nodeId".into(), Value::String(get_str(args, "nodeId")));
            if let Some(raw) = args.get("types").cloned() {
                params.insert("types".into(), raw);
            }
            (vec![], params)
        }

        // get_screenshot
        "get_screenshot" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            let f = get_str(args, "format");
            if !f.is_empty() { params.insert("format".into(), Value::String(f)); }
            if let Some(s) = get_f64(args, "scale") {
                if s > 0.0 { params.insert("scale".into(), serde_json::json!(s)); }
            }
            (ids, params)
        }

        // get_annotations
        "get_annotations" => {
            let mut params = serde_json::Map::new();
            let id = get_str(args, "nodeId");
            if !id.is_empty() { params.insert("nodeId".into(), Value::String(id)); }
            (vec![], params)
        }

        // export_tokens
        "export_tokens" => {
            let mut params = serde_json::Map::new();
            let f = get_str(args, "format");
            if !f.is_empty() { params.insert("format".into(), Value::String(f)); }
            (vec![], params)
        }

        // create_frame, create_rectangle, create_ellipse, create_text
        // set_auto_layout — pass all arguments as params
        "create_frame" | "create_rectangle" | "create_ellipse" | "create_text"
        | "create_paint_style" | "create_text_style" | "create_effect_style" | "create_grid_style"
        | "update_paint_style" | "delete_style"
        | "create_variable_collection" | "add_variable_mode" | "create_variable"
        | "set_variable_value" | "delete_variable" => {
            (vec![], args.clone())
        }

        // import_image
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
            (vec![], params)
        }

        // create_component
        "create_component" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            (vec![node_id], params)
        }

        // create_section
        "create_section" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            copy_opt_f64(args, &mut params, "x");
            copy_opt_f64(args, &mut params, "y");
            copy_opt_f64(args, &mut params, "width");
            copy_opt_f64(args, &mut params, "height");
            (vec![], params)
        }

        // set_text
        "set_text" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("text".into(), Value::String(get_str(args, "text")));
            (vec![node_id], params)
        }

        // set_fills, set_strokes
        "set_fills" | "set_strokes" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("color".into(), args.get("color").cloned().unwrap_or(Value::Null));
            copy_opt_f64(args, &mut params, "opacity");
            copy_opt_f64(args, &mut params, "strokeWeight");
            copy_opt_str(args, &mut params, "mode");
            (vec![node_id], params)
        }

        // move_nodes, resize_nodes, delete_nodes, set_visible, lock_nodes, unlock_nodes
        // rotate_nodes, reorder_nodes, set_blend_mode, set_constraints, batch_rename_nodes
        // ungroup_nodes, group_nodes, detach_instance
        "move_nodes" | "resize_nodes" | "delete_nodes" | "set_visible" | "lock_nodes"
        | "unlock_nodes" | "rotate_nodes" | "reorder_nodes" | "set_blend_mode"
        | "set_constraints" | "batch_rename_nodes" | "ungroup_nodes" | "detach_instance" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            match tool {
                "move_nodes" => { copy_opt_f64(args, &mut params, "x"); copy_opt_f64(args, &mut params, "y"); }
                "resize_nodes" => { copy_opt_f64(args, &mut params, "width"); copy_opt_f64(args, &mut params, "height"); }
                "set_visible" => { if let Some(v) = get_bool(args, "visible") { params.insert("visible".into(), Value::Bool(v)); } }
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
            (ids, params)
        }

        // group_nodes
        "group_nodes" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            (ids, params)
        }

        // rename_node
        "rename_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("name".into(), Value::String(get_str(args, "name")));
            (vec![node_id], params)
        }

        // clone_node
        "clone_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            copy_opt_f64(args, &mut params, "x");
            copy_opt_f64(args, &mut params, "y");
            copy_opt_str(args, &mut params, "parentId");
            (vec![node_id], params)
        }

        // set_opacity
        "set_opacity" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            if let Some(v) = get_f64(args, "opacity") { params.insert("opacity".into(), serde_json::json!(v)); }
            (ids, params)
        }

        // set_corner_radius
        "set_corner_radius" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            for k in &["cornerRadius", "topLeftRadius", "topRightRadius", "bottomLeftRadius", "bottomRightRadius"] {
                copy_opt_f64(args, &mut params, k);
            }
            (ids, params)
        }

        // set_auto_layout
        "set_auto_layout" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            (vec![node_id], args.clone())
        }

        // reparent_nodes
        "reparent_nodes" => {
            let ids: Vec<String> = get_str_array(args, "nodeIds").into_iter().map(|s| normalize_node_id(&s)).collect();
            let mut params = serde_json::Map::new();
            let pid = normalize_node_id(&get_str(args, "parentId"));
            params.insert("parentId".into(), Value::String(pid));
            (ids, params)
        }

        // find_replace_text
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
            (node_ids, params)
        }

        // navigate_to_page
        "navigate_to_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "pageId");
            copy_opt_str(args, &mut params, "pageName");
            (vec![], params)
        }

        // swap_component
        "swap_component" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let component_id = normalize_node_id(&get_str(args, "componentId"));
            let mut params = serde_json::Map::new();
            params.insert("componentId".into(), Value::String(component_id));
            (vec![node_id], params)
        }

        // apply_style_to_node
        "apply_style_to_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("styleId".into(), args.get("styleId").cloned().unwrap_or(Value::Null));
            copy_opt_str(args, &mut params, "target");
            (vec![node_id], params)
        }

        // set_effects
        "set_effects" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("effects".into(), args.get("effects").cloned().unwrap_or(Value::Null));
            (vec![node_id], params)
        }

        // bind_variable_to_node
        "bind_variable_to_node" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("variableId".into(), args.get("variableId").cloned().unwrap_or(Value::Null));
            params.insert("field".into(), args.get("field").cloned().unwrap_or(Value::Null));
            (vec![node_id], params)
        }

        // set_reactions
        "set_reactions" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            params.insert("reactions".into(), args.get("reactions").cloned().unwrap_or(Value::Null));
            copy_opt_str(args, &mut params, "mode");
            (vec![node_id], params)
        }

        // remove_reactions
        "remove_reactions" => {
            let node_id = normalize_node_id(&get_str(args, "nodeId"));
            let mut params = serde_json::Map::new();
            if let Some(indices) = args.get("indices").cloned() {
                params.insert("indices".into(), indices);
            }
            (vec![node_id], params)
        }

        // add_page
        "add_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "name");
            if let Some(idx) = get_f64(args, "index") { params.insert("index".into(), serde_json::json!(idx)); }
            (vec![], params)
        }

        // delete_page
        "delete_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "pageId");
            copy_opt_str(args, &mut params, "pageName");
            (vec![], params)
        }

        // rename_page
        "rename_page" => {
            let mut params = serde_json::Map::new();
            copy_opt_str(args, &mut params, "pageId");
            copy_opt_str(args, &mut params, "pageName");
            copy_opt_str(args, &mut params, "newName");
            (vec![], params)
        }

        _ => (vec![], args.clone()),
    }
}

fn copy_opt_str(args: &serde_json::Map<String, Value>, params: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(v) = args.get(key) {
        if v.as_str().map_or(true, |s| !s.is_empty()) {
            params.insert(key.to_string(), v.clone());
        }
    }
}

fn copy_opt_f64(args: &serde_json::Map<String, Value>, params: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(v) = args.get(key).and_then(|v| v.as_f64()) {
        params.insert(key.to_string(), serde_json::json!(v));
    }
}
