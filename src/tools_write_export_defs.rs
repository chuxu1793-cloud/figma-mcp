use rmcp::model::Tool;

use crate::tool_helpers::*;

/// Export tools: export_frames_to_pdf, export_nodes (2 tools)
pub fn write_export_tools() -> Vec<Tool> {
    vec![
        tool("export_frames_to_pdf", "Export multiple frames as a single multi-page PDF file. Each frame becomes one page in order. Ideal for pitch decks, proposals, and slide exports.",
            schema_mixed(&[
                ("nodeIds", arr_s("Ordered list of frame node IDs to export as PDF pages, colon format e.g. '4029:12345'"), true),
                ("outputPath", s("File path to write the PDF to, must end in .pdf (relative to working directory or absolute)"), true),
            ])),

        tool("export_nodes", "Batch export multiple nodes as individual images. Returns base64-encoded image data for each node. Supports PNG, SVG, JPG, and PDF formats.",
            schema_mixed(&[
                ("nodeIds", arr_s("Node IDs to export, colon format e.g. ['4029:12345', '4029:67890']. If empty, exports current selection."), false),
                ("format", s("Export format: PNG (default), SVG, JPG, or PDF"), false),
                ("scale", n("Export scale for raster formats (default 2)"), false),
            ])),
    ]
}
