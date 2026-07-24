use figma_mcp::schema::{normalize_node_id, valid_node_id, validate_rpc};
use serde_json::json;

#[test]
fn test_normalize_node_id_hyphen_to_colon() {
    assert_eq!(normalize_node_id("4029-12345"), "4029:12345");
}

#[test]
fn test_normalize_node_id_already_colon() {
    assert_eq!(normalize_node_id("4029:12345"), "4029:12345");
}

#[test]
fn test_normalize_node_id_compound() {
    assert_eq!(normalize_node_id("I2167:9091;186:1579"), "I2167:9091;186:1579");
}

#[test]
fn test_normalize_node_id_compound_hyphen() {
    assert_eq!(normalize_node_id("I2167-9091;186-1579"), "I2167:9091;186:1579");
}

#[test]
fn test_normalize_node_id_unrecognized() {
    assert_eq!(normalize_node_id("hello-world"), "hello-world");
}

#[test]
fn test_valid_node_id_simple() {
    assert!(valid_node_id("4029:12345"));
}

#[test]
fn test_valid_node_id_compound() {
    assert!(valid_node_id("I2167:9091;186:1579;186:1745"));
}

#[test]
fn test_valid_node_id_hyphen() {
    assert!(!valid_node_id("4029-12345"));
}

#[test]
fn test_valid_node_id_empty() {
    assert!(!valid_node_id(""));
}

#[test]
fn test_validate_get_node_empty() {
    let result = validate_rpc("get_node", &[], &serde_json::Map::new());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "nodeId is required");
}

#[test]
fn test_validate_get_node_valid() {
    let result = validate_rpc("get_node", &["4029:12345".to_string()], &serde_json::Map::new());
    assert!(result.is_ok());
}

#[test]
fn test_validate_get_node_hyphen() {
    let result = validate_rpc("get_node", &["4029-12345".to_string()], &serde_json::Map::new());
    assert!(result.is_err());
}

#[test]
fn test_validate_set_opacity_valid() {
    let mut params = serde_json::Map::new();
    params.insert("opacity".to_string(), json!(0.5));
    let result = validate_rpc("set_opacity", &["4029:12345".to_string()], &params);
    assert!(result.is_ok());
}

#[test]
fn test_validate_set_opacity_out_of_range() {
    let mut params = serde_json::Map::new();
    params.insert("opacity".to_string(), json!(1.5));
    let result = validate_rpc("set_opacity", &["4029:12345".to_string()], &params);
    assert!(result.is_err());
}

#[test]
fn test_validate_set_blend_mode_valid() {
    let mut params = serde_json::Map::new();
    params.insert("blendMode".to_string(), json!("MULTIPLY"));
    let result = validate_rpc("set_blend_mode", &["4029:12345".to_string()], &params);
    assert!(result.is_ok());
}

#[test]
fn test_validate_set_blend_mode_invalid() {
    let mut params = serde_json::Map::new();
    params.insert("blendMode".to_string(), json!("INVALID_MODE"));
    let result = validate_rpc("set_blend_mode", &["4029:12345".to_string()], &params);
    assert!(result.is_err());
}

#[test]
fn test_validate_create_variable_valid() {
    let mut params = serde_json::Map::new();
    params.insert("name".to_string(), json!("Color/Primary"));
    params.insert("collectionId".to_string(), json!("123"));
    params.insert("type".to_string(), json!("COLOR"));
    let result = validate_rpc("create_variable", &[], &params);
    assert!(result.is_ok());
}

#[test]
fn test_validate_create_variable_invalid_type() {
    let mut params = serde_json::Map::new();
    params.insert("name".to_string(), json!("Color/Primary"));
    params.insert("collectionId".to_string(), json!("123"));
    params.insert("type".to_string(), json!("INVALID"));
    let result = validate_rpc("create_variable", &[], &params);
    assert!(result.is_err());
}

#[test]
fn test_validate_search_nodes_empty_query() {
    let result = validate_rpc("search_nodes", &[], &serde_json::Map::new());
    assert!(result.is_err());
}

#[test]
fn test_validate_search_nodes_valid() {
    let mut params = serde_json::Map::new();
    params.insert("query".to_string(), json!("button"));
    let result = validate_rpc("search_nodes", &[], &params);
    assert!(result.is_ok());
}

#[test]
fn test_validate_group_nodes_less_than_two() {
    let result = validate_rpc("group_nodes", &["4029:12345".to_string()], &serde_json::Map::new());
    assert!(result.is_err());
}

#[test]
fn test_validate_group_nodes_valid() {
    let result = validate_rpc("group_nodes", &["4029:12345".to_string(), "4029:12346".to_string()], &serde_json::Map::new());
    assert!(result.is_ok());
}

#[test]
fn test_validate_unknown_tool() {
    let result = validate_rpc("unknown_tool", &[], &serde_json::Map::new());
    assert!(result.is_ok());
}
