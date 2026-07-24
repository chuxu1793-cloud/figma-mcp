use rmcp::model::Tool;
use serde_json::json;

use crate::tool_helpers::*;

/// Export tools: save_screenshots, export_frames_to_pdf (2 tools — get_screenshot is in read tools)
pub fn write_export_tools() -> Vec<Tool> {
    vec![
        tool("save_screenshots", "Export screenshots for multiple nodes and write them to the local filesystem. Returns file metadata (path, size, dimensions) — no base64 in the response. Use get_screenshot instead when you need the image data in memory.",
            schema_mixed(&[
                ("items", arr_o("List of {nodeId, outputPath, format?, scale?} objects", json!({
                    "type": "object",
                    "properties": {
                        "nodeId": {"type": "string", "description": "Node ID in colon format e.g. '4029:12345'"},
                        "outputPath": {"type": "string", "description": "File path to write the image to"},
                        "format": {"type": "string", "description": "Export format: PNG, SVG, JPG, or PDF"},
                        "scale": {"type": "number", "description": "Export scale for raster formats"}
                    },
                    "required": ["nodeId", "outputPath"]
                })), true),
                ("format", s("Default export format: PNG (default), SVG, JPG, or PDF"), false),
                ("scale", n("Default export scale for raster formats (default 2)"), false),
            ])),

        tool("export_frames_to_pdf", "Export multiple frames as a single multi-page PDF file. Each frame becomes one page in order. Ideal for pitch decks, proposals, and slide exports.",
            schema_mixed(&[
                ("nodeIds", arr_s("Ordered list of frame node IDs to export as PDF pages, colon format e.g. '4029:12345'"), true),
                ("outputPath", s("File path to write the PDF to, must end in .pdf (relative to working directory or absolute)"), true),
            ])),
    ]
}
