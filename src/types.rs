use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Sent from the Rust server to the Figma plugin over WebSocket.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BridgeRequest {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(rename = "nodeIds", skip_serializing_if = "Vec::is_empty", default)]
    pub node_ids: Vec<String>,
    #[serde(skip_serializing_if = "params_is_empty_or_none")]
    pub params: Option<serde_json::Map<String, Value>>,
}

fn params_is_empty_or_none(opt: &Option<serde_json::Map<String, Value>>) -> bool {
    opt.as_ref().is_none_or(|m| m.is_empty())
}

/// Received from the Figma plugin over WebSocket.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BridgeResponse {
    #[serde(rename = "type", default)]
    pub msg_type: String,
    #[serde(rename = "requestId", default)]
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub progress: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

/// Wire format for follower → leader /rpc calls.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RPCRequest {
    pub tool: String,
    #[serde(rename = "nodeIds", default, skip_serializing_if = "Vec::is_empty")]
    pub node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "params_is_empty_or_none")]
    pub params: Option<serde_json::Map<String, Value>>,
}

/// Returned by the leader /rpc endpoint.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RPCResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
}

/// Role of this server process.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Unknown,
    Leader,
    Follower,
}

impl Role {
    pub fn name(&self) -> &'static str {
        match self {
            Role::Leader => "LEADER",
            Role::Follower => "FOLLOWER",
            Role::Unknown => "UNKNOWN",
        }
    }
}
