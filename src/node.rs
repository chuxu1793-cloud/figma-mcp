use tokio::sync::RwLock;
use tracing::info;

use crate::follower::Follower;
use crate::leader::Leader;
use crate::schema::{normalize_node_id, validate_rpc};
use crate::types::{BridgeResponse, Role};

/// Node dynamically routes MCP tool calls to either the Leader bridge
/// or the Follower HTTP proxy, depending on the current role.
pub struct Node {
    role: RwLock<Role>,
    ip: String,
    port: u16,
    leader: RwLock<Option<Leader>>,
    follower: Follower,
    version: String,
}

impl Node {
    pub fn new(ip: &str, port: u16, version: &str) -> Self {
        Self {
            ip: ip.to_string(),
            port,
            role: RwLock::new(Role::Unknown),
            leader: RwLock::new(None),
            version: version.to_string(),
            follower: Follower::new(&format!("http://{}:{}", ip, port)),
        }
    }

    pub async fn role(&self) -> Role {
        *self.role.read().await
    }

    pub async fn role_name(&self) -> &'static str {
        self.role().await.name()
    }

    /// Route a request to the appropriate backend.
    /// Validates RPC before routing.
    /// and normalizes node IDs before routing.
    pub async fn send(
        &self,
        tool: &str,
        mut node_ids: Vec<String>,
        params: &mut Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<BridgeResponse, String> {
        // Normalize node IDs
        for id in &mut node_ids {
            *id = normalize_node_id(id);
        }

        // Normalize param keys that contain node IDs
        if let Some(p) = params {
            for key in &["nodeId", "parentId", "componentId"] {
                if let Some(v) = p.get(*key).and_then(|v| v.as_str()).map(|s| s.to_string()) {
                    p.insert((*key).to_string(), serde_json::Value::String(normalize_node_id(&v)));
                }
            }
        }

        // Validate RPC before routing
        let empty_params = serde_json::Map::new();
        let params_ref = params.as_ref().unwrap_or(&empty_params);
        if let Err(e) = validate_rpc(tool, &node_ids, params_ref) {
            return Ok(BridgeResponse {
                error: e,
                ..Default::default()
            });
        }

        let role = *self.role.read().await;

        if role == Role::Leader {
            let leader_guard = self.leader.read().await;
            if let Some(leader) = leader_guard.as_ref() {
                return leader.bridge().send(tool, node_ids, params.clone()).await;
            }
        }

        // Follower or Unknown — proxy to leader
        self.follower.send(tool, node_ids, params.clone()).await
    }

    /// Attempt to become leader. Returns Err if port is in use.
    pub async fn become_leader(&self) -> Result<(), String> {
        {
            let role = self.role.read().await;
            if *role == Role::Leader {
                return Ok(());
            }
        }

        // Create leader outside the lock — port binding may fail
        let mut leader = Leader::new(&self.ip, self.port, &self.version);
        leader.start()?;

        // Atomically update both leader and role
        let mut leader_guard = self.leader.write().await;
        let mut role_guard = self.role.write().await;

        let old_leader = leader_guard.take();
        *leader_guard = Some(leader);
        *role_guard = Role::Leader;

        drop(leader_guard);
        drop(role_guard);

        if let Some(mut old) = old_leader {
            old.stop().await;
        }
        info!("became LEADER");
        Ok(())
    }

    /// Transition to follower role, stopping the leader if running.
    pub async fn become_follower(&self) {
        {
            let role = self.role.read().await;
            if *role == Role::Follower {
                return;
            }
        }

        let mut leader_guard = self.leader.write().await;
        let mut role_guard = self.role.write().await;

        let old_leader = leader_guard.take();
        *role_guard = Role::Follower;

        drop(leader_guard);
        drop(role_guard);

        if let Some(mut old) = old_leader {
            old.stop().await;
        }
        info!("became FOLLOWER");
    }

    /// Shut down the node regardless of role.
    pub async fn stop(&self) {
        let mut leader_guard = self.leader.write().await;
        let mut role_guard = self.role.write().await;

        let old_leader = leader_guard.take();
        *role_guard = Role::Unknown;

        drop(leader_guard);
        drop(role_guard);

        if let Some(mut old) = old_leader {
            old.stop().await;
        }
    }
}
