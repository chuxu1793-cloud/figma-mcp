use std::time::Duration;

use tracing::{debug, warn};

use crate::types::{BridgeResponse, RPCRequest, RPCResponse};

/// Proxies MCP tool calls to the leader via HTTP /rpc.
pub struct Follower {
    leader_url: String,
    client: reqwest::Client,
}

impl Follower {
    pub fn new(leader_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(35))
            .build()
            .expect("failed to build reqwest client");
        Self {
            leader_url: leader_url.to_string(),
            client,
        }
    }

    /// Proxy a tool call to the leader.
    pub async fn send(
        &self,
        tool: &str,
        node_ids: Vec<String>,
        params: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<BridgeResponse, String> {
        debug!("proxy {} → {}/rpc", tool, self.leader_url);

        let rpc_req = RPCRequest {
            tool: tool.to_string(),
            node_ids,
            params,
        };

        let url = format!("{}/rpc", self.leader_url);
        let resp = self
            .client
            .post(&url)
            .json(&rpc_req)
            .send()
            .await
            .map_err(|e| format!("rpc call: {}", e))?;

        let rpc_resp: RPCResponse = resp
            .json()
            .await
            .map_err(|e| format!("unmarshal: {}", e))?;

        if !rpc_resp.error.is_empty() {
            debug!("proxy {} error from leader: {}", tool, rpc_resp.error);
            return Ok(BridgeResponse {
                error: rpc_resp.error,
                ..Default::default()
            });
        }

        debug!("proxy {} ok", tool);
        Ok(BridgeResponse {
            msg_type: tool.to_string(),
            data: rpc_resp.data,
            ..Default::default()
        })
    }

    /// Check if the leader is alive.
    pub async fn ping(&self) -> bool {
        let url = format!("{}/ping", self.leader_url);
        match self
            .client
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => {
                let ok = resp.status().is_success();
                debug!("ping {} → {} (healthy={})", self.leader_url, resp.status(), ok);
                ok
            }
            Err(e) => {
                warn!("ping {} failed: {}", self.leader_url, e);
                false
            }
        }
    }
}
