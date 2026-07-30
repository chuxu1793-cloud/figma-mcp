use rmcp::model::Tool;

use crate::tool_helpers::*;

/// Export tools: export_frames_to_pdf (1 tool — get_screenshot is in read tools)
pub fn write_export_tools() -> Vec<Tool> {
    vec![
        tool("export_frames_to_pdf", "Export multiple frames as a single multi-page PDF file. Each frame becomes one page in order. Ideal for pitch decks, proposals, and slide exports.",
            schema_mixed(&[
                ("nodeIds", arr_s("Ordered list of frame node IDs to export as PDF pages, colon format e.g. '4029:12345'"), true),
                ("outputPath", s("File path to write the PDF to, must end in .pdf (relative to working directory or absolute)"), true),
            ])),
    ]
}
