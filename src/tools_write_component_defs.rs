use rmcp::model::Tool;

use crate::tool_helpers::*;

/// Component tools: navigate_to_page, group_nodes, ungroup_nodes, swap_component, detach_instance (5 tools)
pub fn write_component_tools() -> Vec<Tool> {
    vec![
        tool("navigate_to_page", "Switch the active Figma page. Provide either pageId or pageName.",
            schema_mixed(&[
                ("pageId", s("Page node ID in colon format e.g. '0:1'"), false),
                ("pageName", s("Exact page name to navigate to"), false),
            ])),

        tool("group_nodes", "Group two or more nodes into a GROUP. All nodes must share the same parent.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs to group (minimum 2), in colon format e.g. ['4029:12345', '4029:12346']"), true),
                ("name", s("Optional name for the new group"), false),
            ])),

        tool("ungroup_nodes", "Ungroup one or more GROUP nodes, moving their children to the parent and removing the group.",
            schema_mixed(&[("nodeIds", arr_s("GROUP node IDs in colon format e.g. ['4029:12345']"), true)])),

        tool("swap_component", "Swap the main component of an existing INSTANCE node, replacing it with a different component while keeping position and size.",
            schema_mixed(&[
                ("nodeId", s("INSTANCE node ID in colon format e.g. 4029:12345"), true),
                ("componentId", s("Target COMPONENT node ID in colon format (from get_local_components)"), true),
            ])),

        tool("detach_instance", "Detach one or more component instances, converting them to plain frames. The link to the main component is broken; all visual properties are preserved.",
            schema_mixed(&[("nodeIds", arr_s("INSTANCE node IDs in colon format e.g. ['4029:12345']"), true)])),
    ]
}
