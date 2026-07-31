use rmcp::model::Tool;
use serde_json::{Map, Value};

/// Build (node_ids, params) from MCP arguments for a specific tool.
pub type BuildParamsFn = fn(&Map<String, Value>) -> (Vec<String>, Map<String, Value>);

/// Validate node_ids and params for a specific tool.
pub type ValidateFn = fn(&[String], &Map<String, Value>) -> Result<(), String>;

/// A tool registration bundles the MCP tool definition with its
/// parameter builder and validator, so adding a new tool requires
/// only one entry in the appropriate `tools_*_defs.rs` module.
pub struct ToolRegistration {
    /// The MCP tool definition (name, description, input schema)
    pub tool: Tool,
    /// Build (node_ids, params) from MCP arguments for this tool.
    pub build_params: BuildParamsFn,
    /// Validate node_ids and params for this tool.
    /// Returns Ok(()) if valid, Err(message) if invalid.
    pub validate: ValidateFn,
}

impl ToolRegistration {
    /// Convenience constructor for tools with no validation needed
    /// (passthrough tools whose validation is handled by the plugin).
    pub fn passthrough(tool: Tool, build_params: BuildParamsFn) -> Self {
        Self {
            tool,
            build_params,
            validate: |_, _| Ok(()),
        }
    }
}
