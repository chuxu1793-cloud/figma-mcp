use rmcp::model::Tool;

use crate::tool_helpers::*;

/// Page tools: navigate_to_page, add_page, delete_page, rename_page (4 tools)
pub fn write_page_tools() -> Vec<Tool> {
    vec![
        tool("navigate_to_page", "Switch the active Figma page. Provide either pageId or pageName.",
            schema_mixed(&[
                ("pageId", s("Page node ID in colon format e.g. '0:1'"), false),
                ("pageName", s("Exact page name to navigate to"), false),
            ])),

        tool("add_page", "Add a new page to the Figma document.",
            schema_mixed(&[
                ("name", s("Name for the new page (default 'Page')"), false),
                ("index", n("Position index to insert the page (0 = first). Defaults to last position."), false),
            ])),

        tool("delete_page", "Delete a page from the Figma document. Cannot delete the only remaining page.",
            schema_mixed(&[
                ("pageId", s("Page node ID in colon format e.g. '0:2'"), false),
                ("pageName", s("Exact page name to delete (alternative to pageId)"), false),
            ])),

        tool("rename_page", "Rename an existing page in the Figma document.",
            schema_mixed(&[
                ("pageId", s("Page node ID in colon format e.g. '0:2'"), false),
                ("pageName", s("Current page name to find (alternative to pageId)"), false),
                ("newName", s("New name for the page"), true),
            ])),
    ]
}
