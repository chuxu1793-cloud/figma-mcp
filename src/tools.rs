use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, JsonObject,
};
use serde_json::Value;
use std::sync::Arc;

use crate::types::BridgeResponse;

/// Convert a BridgeResponse + error into a CallToolResult.
pub fn render_response(resp: Result<BridgeResponse, String>) -> CallToolResult {
    match resp {
        Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        Ok(resp) => {
            if !resp.error.is_empty() {
                CallToolResult::error(vec![ContentBlock::text(resp.error)])
            } else {
                let text = serde_json::to_string(&resp.data.unwrap_or(Value::Null))
                    .unwrap_or_else(|e| format!("marshal response: {}", e));
                CallToolResult::success(vec![ContentBlock::text(text)])
            }
        }
    }
}

/// Convert a JSON value array of strings to Vec<String>.
pub fn to_string_slice(raw: &[Value]) -> Vec<String> {
    raw.iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

/// Build an empty JSON schema (for tools with no parameters).
pub fn empty_schema() -> Arc<JsonObject> {
    Arc::new(
        serde_json::from_str(r#"{"type":"object","properties":{}}"#).unwrap_or_default(),
    )
}

/// Build a JSON schema from a serde_json::Map.
pub fn schema_from_params(properties: serde_json::Map<String, Value>) -> Arc<JsonObject> {
    let mut schema = serde_json::Map::new();
    schema.insert("type".to_string(), Value::String("object".into()));
    schema.insert("properties".to_string(), Value::Object(properties));
    schema.insert("additionalProperties".to_string(), Value::Bool(true));
    Arc::new(schema)
}

/// Helper to extract arguments from CallToolRequestParams as a serde_json::Map.
pub fn extract_arguments(params: &CallToolRequestParams) -> serde_json::Map<String, Value> {
    params
        .arguments
        .clone()
        .unwrap_or_default()
}

/// Helper to get a string argument.
pub fn get_str(args: &serde_json::Map<String, Value>, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Helper to get a f64 argument.
pub fn get_f64(args: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

/// Helper to get a bool argument.
pub fn get_bool(args: &serde_json::Map<String, Value>, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| v.as_bool())
}

/// Helper to get a string array argument.
pub fn get_str_array(args: &serde_json::Map<String, Value>, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

// ── save_screenshots helpers ──

pub fn resolve_output_path(output_path: &str, work_dir: &str) -> Result<String, String> {
    let resolved = if std::path::Path::new(output_path).is_absolute() {
        std::path::Path::new(output_path)
            .components()
            .collect::<std::path::PathBuf>()
            .to_string_lossy()
            .to_string()
    } else {
        std::path::Path::new(work_dir)
            .join(output_path)
            .to_string_lossy()
            .to_string()
    };

    must_be_inside_dir(&resolved, work_dir)
}

fn must_be_inside_dir(resolved: &str, work_dir: &str) -> Result<String, String> {
    let resolved_path = std::path::Path::new(resolved);
    let work_dir_path = std::path::Path::new(work_dir);

    let rel = resolved_path
        .strip_prefix(work_dir_path)
        .map_err(|_| format!("outputPath must be inside the working directory: {}", work_dir))?;

    let rel_str = rel.to_string_lossy();
    if rel_str.starts_with("..") {
        return Err(format!("outputPath must be inside the working directory: {}", work_dir));
    }

    Ok(resolved.to_string())
}

pub fn infer_format(path: &str) -> &str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_lowercase().as_str() {
        "png" => "PNG",
        "svg" => "SVG",
        "jpg" | "jpeg" => "JPG",
        "pdf" => "PDF",
        _ => "",
    }
}

pub fn coalesce<'a>(a: &'a str, b: &'a str) -> &'a str {
    if !a.is_empty() {
        a
    } else {
        b
    }
}

pub fn write_base64(b64: &str, output_path: &str) -> Result<usize, String> {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("base64 decode: {}", e))?;

    if let Some(parent) = std::path::Path::new(output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;
    }

    // O_EXCL — don't overwrite existing files
    let result = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path);

    let mut file = match result {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(format!("file already exists at outputPath: {}", output_path));
        }
        Err(e) => return Err(e.to_string()),
    };

    use std::io::Write;
    file.write_all(&data).map_err(|e| e.to_string())?;
    Ok(data.len())
}
