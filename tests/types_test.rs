use figma_mcp::types::{BridgeRequest, BridgeResponse, RPCRequest, RPCResponse, Role};

#[test]
fn test_bridge_request_serialization() {
    let req = BridgeRequest {
        msg_type: "get_node".to_string(),
        request_id: "req-123".to_string(),
        node_ids: vec!["4029:12345".to_string()],
        params: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"type\":\"get_node\""));
    assert!(json.contains("\"requestId\":\"req-123\""));
    assert!(json.contains("\"nodeIds\""));
}

#[test]
fn test_bridge_request_skip_empty_node_ids() {
    let req = BridgeRequest {
        msg_type: "get_document".to_string(),
        request_id: "req-456".to_string(),
        node_ids: vec![],
        params: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(!json.contains("nodeIds"));
}

#[test]
fn test_bridge_response_deserialization() {
    let json = r#"{"type":"get_node","requestId":"req-123","data":{"id":"4029:12345","name":"Test"}}"#;
    let resp: BridgeResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.msg_type, "get_node");
    assert_eq!(resp.request_id, "req-123");
    assert!(resp.data.is_some());
    assert!(resp.error.is_empty());
}

#[test]
fn test_bridge_response_error() {
    let json = r#"{"type":"get_node","requestId":"req-123","error":"node not found"}"#;
    let resp: BridgeResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.error, "node not found");
    assert!(resp.data.is_none());
}

#[test]
fn test_bridge_response_progress() {
    let json = r#"{"type":"get_document","requestId":"req-789","progress":50,"message":"Loading..."}"#;
    let resp: BridgeResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.progress, 50);
    assert_eq!(resp.message, "Loading...");
}

#[test]
fn test_rpc_request_serialization() {
    let req = RPCRequest {
        tool: "get_node".to_string(),
        node_ids: vec!["4029:12345".to_string()],
        params: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"tool\":\"get_node\""));
}

#[test]
fn test_rpc_response_serialization() {
    let resp = RPCResponse {
        data: Some(serde_json::json!({"id": "123"})),
        error: String::new(),
    };

    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"data\""));
    assert!(!json.contains("\"error\"")); // empty error skipped
}

#[test]
fn test_rpc_response_error_only() {
    let resp = RPCResponse {
        data: None,
        error: "something went wrong".to_string(),
    };

    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"error\":\"something went wrong\""));
    assert!(!json.contains("\"data\""));
}

#[test]
fn test_role_name() {
    assert_eq!(Role::Leader.name(), "LEADER");
    assert_eq!(Role::Follower.name(), "FOLLOWER");
    assert_eq!(Role::Unknown.name(), "UNKNOWN");
}

#[test]
fn test_role_equality() {
    assert_eq!(Role::Leader, Role::Leader);
    assert_ne!(Role::Leader, Role::Follower);
    assert_ne!(Role::Follower, Role::Unknown);
}
