use rmcp::model::Tool;

use crate::tool_helpers::*;

/// Write-create tools: create_frame, create_rectangle, create_ellipse, create_text, import_image, create_component, create_section (7 tools)
pub fn write_create_tools() -> Vec<Tool> {
    vec![
        tool("create_frame", "Create a new frame on the current page or inside a parent node.",
            schema_mixed(&[
                ("x", n("X position (default 0)"), false),
                ("y", n("Y position (default 0)"), false),
                ("width", n("Width in pixels (default 100)"), false),
                ("height", n("Height in pixels (default 100)"), false),
                ("name", s("Frame name"), false),
                ("fillColor", s("Fill color as hex e.g. #FFFFFF"), false),
                ("layoutMode", s("Auto-layout direction: HORIZONTAL, VERTICAL, or NONE"), false),
                ("paddingTop", n("Auto-layout top padding"), false),
                ("paddingRight", n("Auto-layout right padding"), false),
                ("paddingBottom", n("Auto-layout bottom padding"), false),
                ("paddingLeft", n("Auto-layout left padding"), false),
                ("itemSpacing", n("Auto-layout gap between children"), false),
                ("primaryAxisAlignItems", s("Main-axis alignment: MIN, CENTER, MAX, or SPACE_BETWEEN"), false),
                ("counterAxisAlignItems", s("Cross-axis alignment: MIN, CENTER, MAX, or BASELINE"), false),
                ("primaryAxisSizingMode", s("Main-axis sizing: FIXED or AUTO (hug)"), false),
                ("counterAxisSizingMode", s("Cross-axis sizing: FIXED or AUTO (hug)"), false),
                ("layoutWrap", s("Wrap behaviour: NO_WRAP or WRAP"), false),
                ("counterAxisSpacing", n("Gap between wrapped rows/columns (only when layoutWrap is WRAP)"), false),
                ("parentId", s("Parent node ID in colon format. Defaults to current page."), false),
            ])),

        tool("create_rectangle", "Create a new rectangle on the current page or inside a parent node.",
            schema_mixed(&[
                ("x", n("X position (default 0)"), false),
                ("y", n("Y position (default 0)"), false),
                ("width", n("Width in pixels (default 100)"), false),
                ("height", n("Height in pixels (default 100)"), false),
                ("name", s("Rectangle name"), false),
                ("fillColor", s("Fill color as hex e.g. #FF5733"), false),
                ("cornerRadius", n("Corner radius in pixels"), false),
                ("parentId", s("Parent node ID in colon format. Defaults to current page."), false),
            ])),

        tool("create_ellipse", "Create a new ellipse (circle/oval) on the current page or inside a parent node.",
            schema_mixed(&[
                ("x", n("X position (default 0)"), false),
                ("y", n("Y position (default 0)"), false),
                ("width", n("Width in pixels (default 100)"), false),
                ("height", n("Height in pixels (default 100)"), false),
                ("name", s("Ellipse name"), false),
                ("fillColor", s("Fill color as hex e.g. #3B82F6"), false),
                ("parentId", s("Parent node ID in colon format. Defaults to current page."), false),
            ])),

        tool("create_text", "Create a new text node on the current page or inside a parent node. The font is loaded automatically before insertion. Returns the created node ID and bounds. Use set_text to update the content of an existing text node.",
            schema_mixed(&[
                ("text", s("Text content to display"), true),
                ("x", n("X position in pixels (default 0)"), false),
                ("y", n("Y position in pixels (default 0)"), false),
                ("fontSize", n("Font size in pixels (default 14)"), false),
                ("fontFamily", s("Font family name e.g. 'Inter', 'Roboto', 'SF Pro Display' (default Inter). Must be a font installed in Figma."), false),
                ("fontStyle", s("Font style variant e.g. 'Regular', 'Bold', 'Italic', 'Medium', 'SemiBold' (default Regular). Must match an available style for the chosen fontFamily."), false),
                ("fillColor", s("Text color as hex e.g. #000000 (default black)"), false),
                ("name", s("Node name shown in the layers panel (defaults to the text content)"), false),
                ("parentId", s("Parent node ID in colon format. Defaults to current page."), false),
            ])),

        tool("import_image", "Import a base64-encoded image into Figma as a rectangle with an image fill. Use get_screenshot to capture images or provide your own base64 PNG/JPG.",
            schema_mixed(&[
                ("imageData", s("Base64-encoded image data (PNG or JPG)"), true),
                ("x", n("X position (default 0)"), false),
                ("y", n("Y position (default 0)"), false),
                ("width", n("Width in pixels (default 200)"), false),
                ("height", n("Height in pixels (default 200)"), false),
                ("name", s("Node name"), false),
                ("scaleMode", s("Image scale mode: FILL (default), FIT, CROP, or TILE"), false),
                ("parentId", s("Parent node ID in colon format. Defaults to current page."), false),
            ])),

        tool("create_component", "Convert an existing FRAME node into a reusable COMPONENT. The frame is replaced in place by the new component.",
            schema_mixed(&[
                ("nodeId", s("FRAME node ID to convert, in colon format e.g. '4029:12345'"), true),
                ("name", s("Optional name for the component. Defaults to the frame's current name."), false),
            ])),

        tool("create_section", "Create a Figma Section node on the current page. Sections are the modern way to organize frames and groups on a page.",
            schema_mixed(&[
                ("name", s("Section name (default 'Section')"), false),
                ("x", n("X position (default 0)"), false),
                ("y", n("Y position (default 0)"), false),
                ("width", n("Width in pixels"), false),
                ("height", n("Height in pixels"), false),
            ])),
    ]
}
