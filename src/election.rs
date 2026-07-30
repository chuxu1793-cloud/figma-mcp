use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use tracing::info;

use crate::follower::Follower;
use crate::node::Node;
use crate::types::Role;

/// Election determines the initial role and monitors leader health.
/// If the leader dies, a follower will attempt a takeover.
pub struct Election {
    node: Arc<Node>,
    follower: Arc<Follower>,
    shutdown: Option<Arc<Notify>>,
}

impl Election {
    pub fn new(ip: &str, port: u16, node: Arc<Node>) -> Self {
        Self {
            node,
            follower: Arc::new(Follower::new(&format!("http://{}:{}", ip, port))),
            shutdown: None,
        }
    }

    /// Determine initial role and launch the background monitor.
    pub async fn start(&mut self) -> Result<(), String> {
        self.determine_role().await?;

        let notify = Arc::new(Notify::new());
        self.shutdown = Some(notify.clone());

        let node = self.node.clone();
        let follower = self.follower.clone();

        tokio::spawn(async move {
            loop {
                let jitter = election_jitter();
                tokio::select! {
                    _ = tokio::time::sleep(jitter) => {}
                    _ = notify.notified() => { return; }
                }

                let role = node.role().await;
                match role {
                    Role::Follower => {
                        if !follower.ping().await {
                            info!("leader not responding, attempting takeover...");
                            if let Err(e) = node.become_leader().await {
                                info!("takeover failed: {}", e);
                            }
                        }
                    }
                    Role::Unknown => {
                        if node.become_leader().await.is_ok() {
                            // ok
                        } else if follower.ping().await {
                            node.become_follower().await;
                        } else {
                            info!("port taken but leader not responding — will retry");
                        }
                    }
                    Role::Leader => {
                        // Nothing — we are the leader
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the background monitor.
    pub fn stop(&mut self) {
        if let Some(notify) = &self.shutdown {
            notify.notify_one();
        }
    }

    async fn determine_role(&self) -> Result<(), String> {
        if self.node.become_leader().await.is_ok() {
            return Ok(());
        }

        if self.follower.ping().await {
            self.node.become_follower().await;
            return Ok(());
        }

        info!("port taken but leader not responding — will retry");
        Ok(())
    }
}

/// Election monitor jitter delay.
/// Defaults to 3000–5000ms.
/// Overridable via `FIGMA_MCP_ELECTION_JITTER_MIN` and `FIGMA_MCP_ELECTION_JITTER_MAX` env vars (milliseconds).
fn election_jitter() -> Duration {
    let min = env_u64("FIGMA_MCP_ELECTION_JITTER_MIN", 3000);
    let max = env_u64("FIGMA_MCP_ELECTION_JITTER_MAX", 5000);
    let bound = if max > min { max - min } else { 1 };
    Duration::from_millis(min + fastrand::u64(0..bound))
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
