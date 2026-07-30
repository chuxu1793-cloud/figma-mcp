use rmcp::model::Tool;
use serde_json::json;

use crate::tool_helpers::*;

/// Prototype tools: set_reactions, remove_reactions (2 tools)
pub fn write_prototype_tools() -> Vec<Tool> {
    vec![
        tool("set_reactions", r#"Set prototype reactions on a node. Use mode "replace" (default) to overwrite all reactions, or "append" to add to existing ones.

Supported triggers: ON_CLICK, ON_HOVER, ON_PRESS, ON_DRAG, AFTER_TIMEOUT, MOUSE_ENTER, MOUSE_LEAVE, MOUSE_UP, MOUSE_DOWN
Supported action types: NODE (navigation), BACK, CLOSE, URL
  NODE navigation values: NAVIGATE, OVERLAY, SCROLL_TO, SWAP, CHANGE_TO
Transition types: DISSOLVE, SMART_ANIMATE, MOVE_IN, MOVE_OUT, PUSH, SLIDE_IN, SLIDE_OUT
  DISSOLVE / SMART_ANIMATE: {"type":"DISSOLVE","duration":0.3,"easing":{"type":"EASE_OUT"}}
  Directional (PUSH, MOVE_IN, MOVE_OUT, SLIDE_IN, SLIDE_OUT): also require "direction" (LEFT|RIGHT|TOP|BOTTOM) and "matchLayers" (bool).

Example — on-click navigate with dissolve:
{"trigger":{"type":"ON_CLICK"},"actions":[{"type":"NODE","destinationId":"1:3","navigation":"NAVIGATE","transition":{"type":"DISSOLVE","duration":0.3,"easing":{"type":"EASE_OUT"}},"preserveScrollPosition":false}]}

Example — go back on click:
{"trigger":{"type":"ON_CLICK"},"actions":[{"type":"BACK"}]}"#,
            schema_mixed(&[
                ("nodeId", s("Node ID in colon format e.g. '4029:12345'"), true),
                ("reactions", arr_o("Array of reaction objects. Each has a 'trigger' and an 'actions' array of Action objects.", json!({
                    "type": "object",
                    "properties": {
                        "trigger": {
                            "type": "object",
                            "description": "Trigger object",
                            "properties": {
                                "type": {"type": "string", "description": "Trigger type: ON_CLICK, ON_HOVER, ON_PRESS, ON_DRAG, AFTER_TIMEOUT, MOUSE_ENTER, MOUSE_LEAVE, MOUSE_UP, MOUSE_DOWN"},
                                "timeout": {"type": "number", "description": "Timeout in ms (AFTER_TIMEOUT only)"}
                            },
                            "required": ["type"]
                        },
                        "actions": {
                            "type": "array",
                            "description": "Array of action objects",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "type": {"type": "string", "description": "Action type: NODE, BACK, CLOSE, or URL"},
                                    "destinationId": {"type": "string", "description": "Target node ID (NODE only)"},
                                    "navigation": {"type": "string", "description": "Navigation type: NAVIGATE, OVERLAY, SCROLL_TO, SWAP, CHANGE_TO (NODE only)"},
                                    "transition": {"type": "object", "description": "Transition object with type, duration, easing, and optionally direction/matchLayers"},
                                    "preserveScrollPosition": {"type": "boolean", "description": "Preserve scroll position (NODE only)"},
                                    "url": {"type": "string", "description": "URL to open (URL only)"}
                                },
                                "required": ["type"]
                            }
                        }
                    },
                    "required": ["trigger", "actions"]
                })), true),
                ("mode", s(r#""replace" (default) overwrites all existing reactions; "append" adds to them"#), false),
            ])),

        tool("remove_reactions", "Remove prototype reactions from a node. Omit indices to remove all reactions. Provide a zero-based indices array to remove specific reactions (use get_reactions first to see current indices).",
            schema_mixed(&[
                ("nodeId", s("Node ID in colon format e.g. '4029:12345'"), true),
                ("indices", arr_o("Zero-based indices of reactions to remove. Omit or pass [] to remove all.", json!({"type": "number"})), false),
            ])),
    ]
}
