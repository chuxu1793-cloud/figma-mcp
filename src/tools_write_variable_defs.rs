use rmcp::model::Tool;
use serde_json::json;

use crate::tool_helpers::*;

/// Variable tools: create_variable_collection, add_variable_mode, create_variable, set_variable_value, delete_variable (5 tools)
pub fn write_variable_tools() -> Vec<Tool> {
    vec![
        tool("create_variable_collection", "Create a new local variable collection with an optional initial mode name. NOTE — Figma free plan limits each collection to 1 mode. If you need Light/Dark (or any multi-mode) theming and the user is on the free plan, do NOT try to call add_variable_mode; instead use the name-prefix workaround: create all variables in a single collection and prefix each variable name with its mode, e.g. 'light/color-bg' and 'dark/color-bg'. Inform the user of this limitation.",
            schema_mixed(&[
                ("name", s("Collection name"), true),
                ("initialModeName", s("Name for the initial mode (default 'Mode 1')"), false),
            ])),

        tool("add_variable_mode", "Add a new mode to an existing variable collection (e.g. Light/Dark, Desktop/Mobile). IMPORTANT — Figma free plan only allows 1 mode per collection; calling this tool on a free-plan account will return the error 'Limited to 1 modes only'. If that error occurs, stop retrying and switch to the name-prefix workaround: keep the single default mode and create variables prefixed by mode, e.g. 'light/color-bg' and 'dark/color-bg' in the same collection. Tell the user that native multi-mode variables require a paid Figma plan (Professional or above).",
            schema_mixed(&[
                ("collectionId", s("Variable collection ID"), true),
                ("modeName", s("Name for the new mode"), true),
            ])),

        tool("create_variable", "Create a new variable (design token) inside an existing collection. Returns the new variable's ID. Use get_variable_defs to find collection IDs, set_variable_value to set values per mode, and bind_variable_to_node to apply the variable to a node property.",
            schema_mixed(&[
                ("name", s("Variable name — use slash notation to group e.g. 'Color/Primary', 'Spacing/MD'"), true),
                ("collectionId", s("ID of the variable collection to add this variable to (from get_variable_defs)"), true),
                ("type", s("Variable type: COLOR (hex color), FLOAT (numeric dimension/spacing), STRING (text), or BOOLEAN (true/false toggle)"), true),
                ("value", s("Initial value for the first mode. COLOR: hex e.g. #FF5733. FLOAT: number e.g. 16. STRING: text. BOOLEAN: true or false."), false),
            ])),

        tool("set_variable_value", "Set a variable's value for a specific mode.",
            schema_mixed(&[
                ("variableId", s("Variable ID"), true),
                ("modeId", s("Mode ID within the collection"), true),
                ("value", s("Value to set. COLOR: hex e.g. #FF5733. FLOAT: number e.g. 16. STRING: text. BOOLEAN: true or false."), true),
            ])),

        tool("delete_variable", "Delete a single variable (provide variableId) or an entire collection and all its variables (provide collectionId). Provide exactly one of the two — not both.",
            schema_mixed(&[
                ("variableId", s("Variable ID to delete"), false),
                ("collectionId", s("Collection ID to delete (removes all variables in the collection)"), false),
            ])),
    ]
}
