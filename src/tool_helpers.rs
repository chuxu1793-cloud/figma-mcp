use rmcp::model::{JsonObject, Tool};
use serde_json::{json, Value};
use std::sync::Arc;

/// Create a tool with a simple JSON schema.
pub fn tool(name: &str, desc: &str, schema: Value) -> Tool {
    let input_schema: Arc<JsonObject> = Arc::new(
        serde_json::from_value(schema).unwrap_or_default(),
    );
    Tool::new(name.to_string(), desc.to_string(), input_schema)
}

/// Empty properties schema for parameter-less tools.
pub fn no_params_schema() -> Value {
    json!({
        "type": "object",
        "properties": {}
    })
}

/// Helper to build a schema with string properties.
pub fn schema(props: &[(&str, &str, bool, &str)]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for (name, prop_type, is_required, description) in props {
        let mut prop = serde_json::Map::new();
        prop.insert("type".to_string(), Value::String((*prop_type).into()));
        prop.insert("description".to_string(), Value::String((*description).into()));
        properties.insert((*name).to_string(), Value::Object(prop));
        if *is_required {
            required.push(Value::String((*name).into()));
        }
    }
    json!({
        "type": "object",
        "properties": Value::Object(properties),
        "required": Value::Array(required),
        "additionalProperties": true
    })
}

/// Helper to build a schema with mixed property types from JSON fragments.
pub fn schema_mixed(props: &[(&str, Value, bool)]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for (name, prop_schema, is_required) in props {
        properties.insert((*name).to_string(), prop_schema.clone());
        if *is_required {
            required.push(Value::String((*name).into()));
        }
    }
    json!({
        "type": "object",
        "properties": Value::Object(properties),
        "required": Value::Array(required),
        "additionalProperties": true
    })
}

pub fn s(desc: &str) -> Value {
    json!({"type": "string", "description": desc})
}

pub fn n(desc: &str) -> Value {
    json!({"type": "number", "description": desc})
}

pub fn b(desc: &str) -> Value {
    json!({"type": "boolean", "description": desc})
}

pub fn arr_s(desc: &str) -> Value {
    json!({"type": "array", "description": desc, "items": {"type": "string"}})
}

pub fn arr_o(desc: &str, item_props: Value) -> Value {
    json!({"type": "array", "description": desc, "items": item_props})
}
