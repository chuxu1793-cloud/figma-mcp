// Auto-generated prompt text constants.
// Each is a raw string literal containing prompt content.
// Using r## delimiters because some prompts contain "# sequences.

pub const READ_DESIGN_STRATEGY: &str = r##"To effectively read a Figma design with figma-mcp:

1. Start with get_metadata — understand file name, pages, and current page
2. Use get_pages to list all pages without loading their full trees
3. Use get_design_context (depth=2, detail=compact) for a token-efficient summary of the current selection or page
   - detail=minimal: id/name/type/bounds only (~5% tokens)
   - detail=compact: + fills/strokes/opacity (~30% tokens)
   - detail=full: everything, default (100% tokens)
   - dedupe_components=true: INSTANCE nodes are collapsed to compact stubs (mainComponentId + componentProperties overrides);
     unique component structures are collected once in a top-level componentDefs map.
     Use this whenever the screen contains repeated component instances (e.g. card lists, table rows, nav items).
     Typical savings: 5–10× fewer tokens vs full serialization of repeated instances.
4. For screens with many repeated components, the recommended reading flow is:
   a. get_design_context(depth=2, detail=minimal, dedupe_components=true) — see the instance layout + component IDs
   b. Inspect componentDefs in the response — one definition per unique component, not one per instance
   c. Read componentProperties on each instance stub — variant selections, text overrides, boolean toggles
   d. Drill into specific instances with get_node only when an instance has unique overrides you need to inspect
5. Use search_nodes to find nodes by name or type without dumping the entire tree
6. Drill into specific nodes with get_nodes_info (prefer batch over single calls)
7. For text-heavy components, use scan_nodes_by_types with ["TEXT"] to collect all copy at once
8. Use scan_nodes_by_types to find all FRAME/COMPONENT/INSTANCE nodes in a subtree
9. Call get_styles and get_variable_defs once per session to understand the design system
10. Call get_fonts to understand typography usage across the page at a glance
11. Use get_viewport to see what the user is currently looking at in the canvas
12. Use get_reactions to inspect prototype interactions on a node
13. Call get_screenshot last and only when visual confirmation is needed — it is expensive
14. Node IDs use colon format: 4029:12345 — never use hyphens
15. get_local_components returns componentSets and variantProperties for variant-aware inspection"##;

pub const DESIGN_STRATEGY: &str = r##"When working with Figma designs, follow these best practices:

1. Start with Document Structure:
   - First use get_metadata() to understand the current document
   - Use get_pages() to list all pages
   - Plan your layout hierarchy before creating elements
   - Create a main container frame for each screen/section

2. Naming Conventions:
   - Use descriptive, semantic names for all elements
   - Follow a consistent naming pattern (e.g., "Login Screen", "Logo Container", "Email Input")
   - Group related elements with meaningful names

3. Layout Hierarchy:
   - Create parent frames first, then add child elements
   - For forms/login screens:
     * Start with the main screen container frame
     * Create a logo container at the top
     * Group input fields in their own containers
     * Place action buttons (login, submit) after inputs
     * Add secondary elements (forgot password, signup links) last

4. Input Fields Structure:
   - Create a container frame for each input field
   - Include a label text above or inside the input
   - Group related inputs (e.g., username/password) together

5. Element Creation:
   - Use create_frame() for containers and input fields
   - Use create_text() for labels, buttons text, and links
   - Set appropriate colors and styles:
     * Use fillColor for backgrounds
     * Use set_strokes() for borders
     * Set proper fontStyle for different text elements

6. Modifying existing elements:
   - Use set_text() to modify text content of a TEXT node
   - Use set_fills() to change background/fill colors
   - Use move_nodes() / resize_nodes() for position and size adjustments

7. Visual Hierarchy:
   - Position elements in logical reading order (top to bottom)
   - Maintain consistent spacing between elements
   - Use appropriate font sizes for different text types:
     * Larger for headings/welcome text
     * Medium for input labels
     * Standard for button text
     * Smaller for helper text/links

8. Best Practices:
   - Verify each creation with get_nodes_info()
   - Use parentId to maintain proper hierarchy
   - Group related elements together in frames
   - Keep consistent spacing and alignment
   - All write operations are undoable via Ctrl/Cmd+Z in Figma

Example Login Screen Structure:
- Login Screen (main frame)
  - Logo Container (frame)
    - Logo (text)
  - Welcome Text (text)
  - Input Container (frame)
    - Email Input (frame)
      - Email Label (text)
      - Email Field (frame)
    - Password Input (frame)
      - Password Label (text)
      - Password Field (frame)
  - Login Button (frame)
    - Button Text (text)
  - Helper Links (frame)
    - Forgot Password (text)
    - Don't have account (text)"##;

pub const TEXT_REPLACEMENT_STRATEGY: &str = r##"# Intelligent Text Replacement Strategy

## 1. Analyze Design & Identify Structure
- Scan text nodes to understand the overall structure of the design
- Use AI pattern recognition to identify logical groupings:
  * Tables (rows, columns, headers, cells)
  * Lists (items, headers, nested lists)
  * Card groups (similar cards with recurring text fields)
  * Forms (labels, input fields, validation text)
  * Navigation (menu items, breadcrumbs)

scan_nodes_by_types(nodeId: "node-id", types: ["TEXT"])
get_nodes_info(nodeIds: ["node-id"])  // optional for extra context

## 2. Strategic Chunking for Complex Designs
- Divide replacement tasks into logical content chunks based on design structure
- Use one of these chunking strategies that best fits the design:
  * Structural Chunking: Table rows/columns, list sections, card groups
  * Spatial Chunking: Top-to-bottom, left-to-right in screen areas
  * Semantic Chunking: Content related to the same topic or functionality
  * Component-Based Chunking: Process similar component instances together

## 3. Progressive Replacement with Verification
- Create a safe copy of the node before bulk replacements
- Replace text chunk by chunk with continuous progress updates
- After each chunk is processed:
  * Export that section with get_screenshot for visual verification
  * Verify text fits properly and maintains design integrity
  * Fix issues before proceeding to the next chunk

clone_node(nodeId: "selected-node-id", x: newX, y: newY)
set_text(nodeId: "node-id", text: "New text")
get_screenshot(nodeIds: ["chunk-node-id"], format: "PNG", scale: 0.5)

## 4. Intelligent Handling for Table Data
- For tabular content:
  * Process one row or column at a time
  * Maintain alignment and spacing between cells
  * Consider conditional formatting based on cell content
  * Preserve header/data relationships

## 5. Smart Text Adaptation
- Adaptively handle text based on container constraints:
  * Auto-detect space constraints and adjust text length
  * Apply line breaks at appropriate linguistic points
  * Maintain text hierarchy and emphasis

## 6. Final Verification & Context-Aware QA
- After all chunks are processed:
  * Export the entire design at reduced scale for final verification
  * Check for cross-chunk consistency issues
  * Verify proper text flow between different sections
  * Ensure design harmony across the full composition

## 7. Chunk-Specific Export Scale Guidelines
- Scale exports appropriately based on chunk size:
  * Small chunks (1-5 elements): scale 1.0
  * Medium chunks (6-20 elements): scale 0.7
  * Large chunks (21-50 elements): scale 0.5
  * Very large chunks (50+ elements): scale 0.3
  * Full design verification: scale 0.2

## Best Practices
- Preserve Design Intent: Always prioritize design integrity
- Structural Consistency: Maintain alignment, spacing, and hierarchy
- Visual Feedback: Verify each chunk visually before proceeding
- Incremental Improvement: Learn from each chunk to improve subsequent ones
- Respect Content Relationships: Keep related content consistent across chunks"##;

pub const ANNOTATION_CONVERSION_STRATEGY: &str = r##"# Automatic Annotation Conversion

## Process Overview
Convert manual annotations (numbered/alphabetical indicators with connected descriptions) to Figma's native annotations:

1. Get selected frame/component information
2. Scan and collect all annotation text nodes
3. Scan target UI elements (components, instances, frames)
4. Match annotations to appropriate UI elements
5. Apply native Figma annotations

## Step 1: Get Selection and Initial Setup
get_selection()
get_annotations(nodeId: "selected-node-id")

## Step 2: Scan Annotation Text Nodes
scan_nodes_by_types(nodeId: "selected-node-id", types: ["TEXT"])
// Markers typically have these characteristics:
// - Short text content (usually single digit/letter)
// - Specific font styles (often bold)
// - Located in a container with "Marker" or "Dot" in the name
// - Have a clear naming pattern (e.g., "1", "2", "3" or "A", "B", "C")

## Step 3: Scan Target UI Elements
scan_nodes_by_types(nodeId: "selected-node-id", types: ["COMPONENT", "INSTANCE", "FRAME"])

## Step 4: Match Annotations to Targets
Match each annotation to its target UI element using these strategies in order of priority:

1. Path-Based Matching:
   - Look at the marker's parent container name in the Figma layer hierarchy
   - Remove any "Marker:" or "Annotation:" prefixes from the parent name
   - Find UI elements that share the same parent name or have it in their path

2. Name-Based Matching:
   - Extract key terms from the annotation description
   - Look for UI elements whose names contain these key terms
   - Particularly effective for form fields, buttons, and labeled components

3. Proximity-Based Matching (fallback):
   - Calculate the center point of the marker using its bounds
   - Find the closest UI element by measuring distances to element centers
   - Use this method when other matching strategies fail

## Step 5: Verify Results
get_annotations(nodeId: "selected-node-id")
get_screenshot(nodeIds: ["selected-node-id"], format: "PNG", scale: 0.5)"##;

pub const SWAP_OVERRIDES_INSTANCES: &str = r##"# Swap Component Instance and Override Strategy

## Overview
Transfer content and property overrides from a source instance to one or more target instances
in Figma, maintaining design consistency while reducing manual work.

## Step-by-Step Process

### 1. Selection Analysis
- Use get_selection() to identify the parent component or selected instances
- For parent components, scan for instances with:
  scan_nodes_by_types(nodeId: "parent-id", types: ["INSTANCE"])
- Identify custom slots by name patterns (e.g. "Custom Slot*" or "Instance Slot")
- Determine which is the source instance (with content to copy) and which are targets

### 2. Inspect Source Instance
- Use get_nodes_info(nodeIds: ["source-instance-id"]) to examine the source instance structure
- Use get_nodes_info(nodeIds: [...]) to batch-inspect multiple instances
- Use scan_nodes_by_types(nodeId: "source-instance-id", types: ["TEXT"]) to capture all text content

### 3. Apply Overrides to Targets
- For text overrides: use set_text(nodeId: "target-text-node-id", text: "copied text")
- For fill overrides: use set_fills(nodeId: "target-node-id", color: "#hexcolor")
- For stroke overrides: use set_strokes(nodeId: "target-node-id", color: "#hexcolor")
- Process targets one at a time or identify patterns to apply systematically

### 4. Verification
- Verify results with get_nodes_info() or get_design_context()
- Confirm text content and style overrides have transferred successfully
- Use get_screenshot() for visual confirmation if needed

## Key Tips
- Use scan_nodes_by_types to enumerate all instances before starting
- When working with multiple targets, check the full selection with get_selection()
- Prefer reading the full node tree of the source first to understand its structure
- Keep related content consistent across all target instances"##;

pub const REACTION_TO_CONNECTOR_STRATEGY: &str = r##"# Strategy: Analyze Figma Prototype Reactions and Map Interaction Flows

## Goal
Process the JSON output from the get_reactions tool to understand prototype flows
and produce a clear, structured map of interactions between screens/nodes.

## Input Data
You will receive JSON data from get_reactions. Each node may contain reactions like:
{
  "trigger": { "type": "ON_CLICK" },
  "action": {
    "type": "NAVIGATE",
    "destinationId": "destination-node-id"
  }
}

## Step-by-Step Process

### 1. Gather Context
- Call get_nodes_info(nodeIds: [...]) on all relevant nodes to get their names and types
- Call get_design_context(depth: 2, detail: "minimal") to understand the page structure

### 2. Filter and Transform Reactions
- Iterate through the get_reactions JSON output
- Keep only reactions where action type implies navigation:
  * NAVIGATE, OPEN_OVERLAY, SWAP_OVERLAY
  * Ignore: CHANGE_TO, CLOSE_OVERLAY, and others without a destinationId
- Extract per reaction:
  * sourceNodeId: the node the reaction belongs to
  * destinationId: action.destinationId
  * actionType: action.type
  * triggerType: trigger.type

### 3. Generate Flow Map
For each valid reaction, create a human-readable description:
- "On click → navigate to [Destination Name]"
- "On drag → open [Destination Name] overlay"
- "On hover → swap to [Destination Name]"

Combine these into a structured flow map grouped by source screen.

### 4. Output Format
Produce a summary like:

Flow Map:
- [Screen A] --ON_CLICK/NAVIGATE--> [Screen B]
- [Screen A] --ON_CLICK/OPEN_OVERLAY--> [Modal C]
- [Screen B] --ON_CLICK/NAVIGATE--> [Screen C]

### 5. Verification
- Use get_screenshot(nodeIds: [...]) on key screens to visually confirm the flow
- Cross-check node names from get_nodes_info with the flow map

## Notes
- Node IDs use colon format: 4029:12345 — never use hyphens
- Use get_reactions on a set of nodes that represent screens or interactive frames
- Focus on NAVIGATE actions for the primary user journey"##;

pub const STYLE_AUDIT_STRATEGY: &str = r##"# Style Audit Strategy

Find all nodes that use raw (unlinked) fill colors, text styles, or effect styles instead of the
design system's named styles or variables. Report findings and optionally fix them.

## Steps

1. **Collect the design system**
   - Call get_styles() to list all local paint, text, effect, and grid styles (note their names and IDs).
   - Call get_variable_defs() to list all local COLOR variables (note their names and IDs).

2. **Scan the design**
   - Call get_design_context() with detail="compact" to get the full node tree.
   - For each node that has a fills, strokes, or textStyle property:
     - If the node's style field shows a named style (e.g. "fillStyle": "Brand/Primary") → already linked, skip.
     - If the node shows a raw fill color (e.g. "fills": [{"type":"SOLID","color":...}]) without a style name → flag it.
     - If a TEXT node shows raw fontFamily/fontSize without a textStyle name → flag it.

3. **Match raw values to existing styles**
   - For each flagged node, check whether the raw hex color matches any existing paint style color.
   - If a match is found → recommend apply_style_to_node() to link the node to that style.
   - If no match is found → note the raw value as a design system gap (a new style may be needed).

4. **Report findings**
   Present a table:
   | Node ID | Node Name | Issue | Raw Value | Matching Style |
   |---------|-----------|-------|-----------|----------------|

5. **Fix (optional, ask user first)**
   For each node with a matching style, call:
     apply_style_to_node(nodeId, styleId, target)
   Batch nodes by styleId to minimize round trips.

## Rules
- Never change a node's visual appearance — only link it to a style that already matches.
- Skip INSTANCE nodes whose overrides intentionally diverge from the main component.
- Process in chunks of 20 nodes at a time when scanning large trees."##;

pub const BULK_RENAME_STRATEGY: &str = r##"# Bulk Rename Strategy

Systematically rename nodes to follow a consistent naming convention without moving or
modifying any visual properties.

## Naming Convention (BEM-style, adapt as needed)

- Screens / pages:         "ScreenName" (PascalCase)
- Section frames:          "Section/Name"
- Component instances:     "ComponentName" (match main component name)
- Containers:              "ComponentName/Container"
- Content groups:          "ComponentName/Content"
- Interactive elements:    "ComponentName/ActionName" (e.g. "Card/CTAButton")
- Text nodes:              "Label", "Title", "Body", "Caption"
- Icon wrappers:           "Icon/IconName"
- Auto-generated Figma names to avoid: "Frame 123", "Rectangle 45", "Group 6"

## Steps

1. **Understand the scope**
   Ask the user: rename the entire page, a specific frame, or just selected nodes?
   - Entire page: use get_design_context() to get the root node ID, then scan_nodes_by_types().
   - Specific frame: use get_nodes_info(nodeIds) to inspect it first.
   - Selection: use get_selection().

2. **Scan target nodes**
   Call scan_nodes_by_types(nodeId, types=["FRAME","GROUP","INSTANCE","TEXT","RECTANGLE","ELLIPSE","VECTOR"])
   to get a flat list of all nodes in scope.

3. **Identify nodes needing rename**
   Flag nodes whose names match Figma's auto-generated patterns:
   - "Frame \d+", "Rectangle \d+", "Group \d+", "Ellipse \d+", "Vector \d+", "Component \d+"
   - Any name the user considers non-descriptive.

4. **Propose names**
   For each flagged node, derive a new name from:
   - Its node type and content (TEXT nodes → use their text content as label).
   - Its position in the hierarchy (child of "Card" frame → "Card/...").
   - Its visual role (if it contains only an icon → "Icon/...").
   - For INSTANCE nodes → use the mainComponent name.
   Show a preview table to the user before applying:
   | Node ID | Current Name | Proposed Name |

5. **Apply renames (after user confirmation)**
   Call rename_node(nodeId, name) for each node.
   Process in batches — do not wait for user confirmation between individual renames once
   the full plan is approved.

## Rules
- Never rename nodes that already follow the convention.
- Never change names of COMPONENT master nodes (only instances and frames).
- Preserve "/" hierarchy separators — do not flatten them.
- If unsure about a name, leave it and flag it for the user to decide."##;

pub const DESIGN_TOKEN_GENERATION_STRATEGY: &str = r##"# Design Token Generation Strategy

Scan an existing design to discover all unique colors, font sizes, spacing values, and radii,
then create a structured variable collection and named styles, and finally link nodes to them.

## Steps

### Phase 1 — Discovery
1. Call get_styles() to check what styles already exist (avoid duplicating them).
2. Call get_variable_defs() to check existing variables.
3. Call get_design_context(detail="compact") to scan the full node tree.
4. Collect unique values:
   - **Colors**: all unique hex fills and stroke colors across nodes.
   - **Font sizes**: all unique fontSize values on TEXT nodes.
   - **Spacing**: all unique itemSpacing, paddingTop/Right/Bottom/Left values on FRAME nodes.
   - **Radii**: all unique cornerRadius values.

### Phase 2 — Token naming
Map discovered values to semantic token names. Use this hierarchy:

**Colors** (variable collection "Primitives"):
- Sort colors by hue/lightness.
- Assign names like "Blue/100", "Blue/200", … "Blue/900", "Neutral/50", "Neutral/900", etc.
- Also create a "Semantic" collection with aliases: "Color/Primary", "Color/Background", "Color/Text", etc.

**Spacing** (variable collection "Spacing"):
- Name by scale: "Spacing/0" (0), "Spacing/1" (4px), "Spacing/2" (8px), "Spacing/3" (12px), …

**Radius** (variable collection "Radius"):
- Name: "Radius/None" (0), "Radius/SM" (4), "Radius/MD" (8), "Radius/LG" (16), "Radius/Full" (9999)

**Typography** (local text styles):
- Name: "Heading/H1", "Heading/H2", "Body/Regular", "Body/Small", "Label/Medium", etc.

Present the full token plan to the user for approval before creating anything.

### Phase 3 — Creation
For each approved token:
- COLOR variables: create_variable_collection() → create_variable(type="COLOR") → set_variable_value()
- FLOAT variables: create_variable_collection() → create_variable(type="FLOAT") → set_variable_value()
- Text styles: create_text_style() with name, fontFamily, fontSize, lineHeight, letterSpacing
- Paint styles: create_paint_style() with name, color

### Phase 4 — Linking (optional, ask user)
After creating tokens, offer to link existing nodes:
- For nodes with raw fill colors that match a new variable → bind_variable_to_node(field="fillColor")
- For TEXT nodes with matching font styles → apply_style_to_node(styleId)

## Multi-mode / Theming (Light & Dark)

### Paid plan (Professional / Organization / Enterprise)
Use Figma's native multi-mode variables:
1. create_variable_collection() — creates the collection with one default mode (e.g. "Light").
2. add_variable_mode() — add the second mode (e.g. "Dark").
3. create_variable() — create each variable once; it exists in all modes automatically.
4. set_variable_value() — call once per mode to set the Light value, then again with the Dark value.

### Free plan workaround (when add_variable_mode returns "Limited to 1 modes only")
Figma's free plan is capped at 1 mode per collection. Detect this error and fall back to
name-encoded modes: keep a **single collection with 1 mode**, and prefix every variable name
with the mode it belongs to.

Naming convention — use a slash-prefix per mode:
- light/color-bg, dark/color-bg
- light/color-text, dark/color-text
- light/color-primary, dark/color-primary

Steps:
1. create_variable_collection() — one collection, accept the single default mode as-is.
2. create_variable(name="light/<token>") — Light variant.
3. create_variable(name="dark/<token>") — Dark variant.
4. set_variable_value() for each variable.

Bind the active-theme variable to nodes (e.g. bind the "light/color-bg" variable for light theme).
When the user wants to switch to dark, rebind nodes to the corresponding "dark/*" variable.
Inform the user that native mode-switching requires a paid Figma plan; with this workaround
they manually choose which prefixed variable to bind.

## Rules
- Never delete or overwrite existing styles/variables — only add new ones.
- Prefer variables over paint styles for colors that will need dark-mode variants.
- Always get user approval on the token plan (Phase 2) before executing Phase 3.
- Process in batches of 20 during linking phase.
- Detect the free-plan mode limit at runtime: if add_variable_mode fails with "Limited to 1 modes only", switch to the name-encoded workaround automatically and inform the user."##;

pub const GENERATE_COLOR_PALETTE: &str = r##"# Generate Color Palette

Given one or more brand colors, generate a full design-system color palette with a primitive
scale and semantic aliases, then create them as Figma variables.

## Input
Ask the user for:
- Primary brand color (hex) — required
- Secondary/accent color (hex) — optional
- Whether to include neutral/gray scale — default yes
- Whether to generate dark mode — default yes

## Color Scale Algorithm
For each brand color, generate a 9-step scale (50, 100, 200, 300, 400, 500, 600, 700, 800, 900)
by varying lightness in HSL space:
- 50  → lightest tint  (~95% lightness)
- 500 → base color (the input hex)
- 900 → darkest shade (~15% lightness)

Show the full color table to the user for review before creating anything.

## Semantic Aliases
After the primitive scale, create semantic tokens that reference primitives.

## Creation Steps
1. create_variable_collection(name="Primitives", modeName="Value")
2. For each color in the scale: create_variable + set_variable_value
3. create_variable_collection(name="Semantic Colors", modeName="Light")
4. add_variable_mode(collectionId, modeName="Dark") — if dark mode requested
5. For each semantic alias: create_variable + set_variable_value

## Rules
- Always show the color table preview before executing creation.
- Use only hex values for variable colors."##;

pub const GENERATE_TYPE_SCALE: &str = r##"# Generate Type Scale

Given a base font family and body size, generate a full typographic scale as Figma text styles.

## Input
Ask the user for:
- Font family (e.g. "Inter") — required
- Base body font size in px (e.g. 16) — required
- Scale ratio: "minor-third" (1.2), "major-third" (1.25), "perfect-fourth" (1.333), "golden" (1.618) — default major-third

## Scale Calculation
Using base size B and ratio R, generate styles from Display/2XL down to Caption.

Round all sizes to nearest integer. Minimum size: 10px.

Show the full table to the user for review before creating anything.

## Creation Steps
1. For each style row, call create_text_style() with name, fontFamily, fontStyle, fontSize, lineHeight, letterSpacing.
2. Skip styles already present with the same name.

## Rules
- Always show the scale preview table before executing.
- Use "Regular" font style for weight 400, "Medium" for 500, "SemiBold" for 600, "Bold" for 700.
- Line height in PIXELS = round(fontSize × lineHeightRatio).
- Letter spacing in PERCENT = letterSpacingEm × 100."##;

pub const GENERATE_COMPONENT_VARIANTS: &str = r##"# Generate Component Variants

Given an existing frame or component, produce a set of visual variants (e.g. sizes, color themes,
states) by cloning and mutating it. Arrange the variants in a tidy grid for review.

## Input
Ask the user:
- Source node ID (the base component or frame to clone)
- What variants to generate — choose one or more:
  a) **Sizes** — Small, Medium, Large
  b) **Color themes** — e.g. Primary, Secondary, Danger, Success, Warning
  c) **States** — Default, Hover, Pressed, Disabled, Loading
  d) **Dark mode** — duplicate with inverted background/text colors
- Arrange output on same page or new frame?

## Steps
1. Inspect the source: get_nodes_info(nodeIds: [sourceNodeId])
2. Plan the variant grid layout
3. Create container frame if requested
4. For each variant: clone_node, then modify (resize_nodes, set_fills, rename_node)
5. Summarize all created node IDs

## Rules
- Always inspect the source node before cloning.
- Never modify the original source node.
- Keep all variants on the same page unless the user requests otherwise.
- Add a text label below each variant showing its name."##;

pub const ANALYZE_DESIGN_SYSTEM: &str = r##"Analyze the current document's design system maturity. Follow these steps:

1. Call get_styles to list all local paint, text, effect, and grid styles. Note any naming inconsistencies or missing group structure.
2. Call get_variable_defs to inspect variable collections and modes. Check for:
   - Color variables that duplicate paint styles (same value in both)
   - Variables without clear naming conventions (no slash-separated groups)
   - Missing modes for theming (e.g. no Light/Dark)
3. Call get_local_components to list all components. Check for:
   - Components without variant properties that should have them (e.g. states, sizes)
   - Component sets with incomplete coverage
   - Naming convention issues
4. Call get_fonts to see all fonts in use. Flag:
   - More than 3 font families (potential inconsistency)
   - Fonts without corresponding text styles
5. Use get_design_context with detail=compact on key screens to check:
   - Nodes using raw fill colors instead of styles/variables
   - Text nodes without text styles applied
   - Inconsistent spacing patterns
6. Provide a maturity score (1-5) for each category: Styles, Variables, Components, Typography, Spacing
7. List the top 5 actionable improvements, prioritized by impact."##;

pub const COMPONENT_AUDIT: &str = r##"Audit component usage across the current document. Follow these steps:

1. Call get_local_components to get all component definitions. Note their IDs and names.
2. Use get_design_context with dedupe_components=true on each page to find all instances:
   - The componentDefs map shows unique component definitions
   - The context tree shows where instances are used and their overrides
3. For each component, identify:
   - How many instances exist (search_nodes by component name)
   - Whether any instances have text overrides (check overrides array in deduped context)
   - Whether any instances have property overrides (componentProperties differ from defaults)
4. Flag issues:
   - Instances with overrides that should be new variants instead
   - Components with zero instances (unused — candidates for deletion)
   - Instances that should be detached (unique overrides making them non-reusable)
   - Missing common variants (e.g. a Button component without disabled/active states)
5. Use scan_nodes_by_types with ['INSTANCE'] on the current page to find all instances
6. For each instance with overrides, use get_nodes_info to inspect the override details
7. Produce a report:
   - Component usage summary (component name → instance count → override count)
   - List of orphaned components (defined but not used)
   - List of instances with overrides that should be promoted to variants
   - Recommended new variants to create"##;
