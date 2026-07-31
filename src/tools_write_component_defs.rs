use rmcp::model::Tool;

use crate::tool_helpers::*;

/// Component tools: group_nodes, ungroup_nodes, swap_component, detach_instance, set_component_property (5 tools)
pub fn write_component_tools() -> Vec<Tool> {
    vec![
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

        tool("set_component_property", "Add a new component property (variant, boolean, or text) to a COMPONENT or COMPONENT_SET. Use this to create variant properties for component sets.",
            schema_mixed(&[
                ("nodeId", s("COMPONENT or COMPONENT_SET node ID in colon format e.g. '4029:12345'"), true),
                ("name", s("Property name e.g. 'State', 'Size', 'Disabled'"), true),
                ("type", s("Property type: BOOLEAN, TEXT, or VARIANT"), true),
                ("defaultValue", s("Default value for the property. BOOLEAN: true/false, TEXT: string, VARIANT: e.g. 'Default'"), false),
            ])),
    ]
}
