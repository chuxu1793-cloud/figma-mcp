use rmcp::model::{GetPromptResult, Prompt, PromptMessage, Role};

/// Returns all 12 MCP prompt definitions.
pub fn all_prompts() -> Vec<Prompt> {
    vec![
        Prompt::new("read_design_strategy", Some("Best practices for reading Figma designs with figma-mcp"), None),
        Prompt::new("design_strategy", Some("Best practices for working with Figma designs"), None),
        Prompt::new("text_replacement_strategy", Some("Systematic approach for replacing text in Figma designs"), None),
        Prompt::new("annotation_conversion_strategy", Some("Strategy for converting manual annotations to Figma's native annotations"), None),
        Prompt::new("swap_overrides_instances", Some("Strategy for transferring overrides between component instances in Figma"), None),
        Prompt::new("reaction_to_connector_strategy", Some("Strategy for analyzing Figma prototype reactions and mapping interaction flows"), None),
        Prompt::new("style_audit_strategy", Some("Audit a design for nodes using raw values instead of linked styles or variables"), None),
        Prompt::new("bulk_rename_strategy", Some("Rename nodes across a design following a naming convention"), None),
        Prompt::new("design_token_generation_strategy", Some("Extract raw values from an existing design and build a structured variable + style token system"), None),
        Prompt::new("generate_color_palette", Some("Generate a complete semantic color palette (primitive scale + semantic aliases) from one or more brand colors"), None),
        Prompt::new("generate_type_scale", Some("Generate a complete typography scale (text styles) from a base font and size"), None),
        Prompt::new("generate_component_variants", Some("Generate design variants of an existing component or frame (size, color, state, theme)"), None),
    ]
}

/// Get the prompt result (messages) for a given prompt name.
pub fn get_prompt_result(name: &str) -> Option<GetPromptResult> {
    let text = match name {
        "read_design_strategy" => crate::prompt_texts::READ_DESIGN_STRATEGY,
        "design_strategy" => crate::prompt_texts::DESIGN_STRATEGY,
        "text_replacement_strategy" => crate::prompt_texts::TEXT_REPLACEMENT_STRATEGY,
        "annotation_conversion_strategy" => crate::prompt_texts::ANNOTATION_CONVERSION_STRATEGY,
        "swap_overrides_instances" => crate::prompt_texts::SWAP_OVERRIDES_INSTANCES,
        "reaction_to_connector_strategy" => crate::prompt_texts::REACTION_TO_CONNECTOR_STRATEGY,
        "style_audit_strategy" => crate::prompt_texts::STYLE_AUDIT_STRATEGY,
        "bulk_rename_strategy" => crate::prompt_texts::BULK_RENAME_STRATEGY,
        "design_token_generation_strategy" => crate::prompt_texts::DESIGN_TOKEN_GENERATION_STRATEGY,
        "generate_color_palette" => crate::prompt_texts::GENERATE_COLOR_PALETTE,
        "generate_type_scale" => crate::prompt_texts::GENERATE_TYPE_SCALE,
        "generate_component_variants" => crate::prompt_texts::GENERATE_COMPONENT_VARIANTS,
        _ => return None,
    };
    Some(GetPromptResult::new(vec![
        PromptMessage::new_text(Role::User, text),
    ]))
}
