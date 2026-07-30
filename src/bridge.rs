use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, info, warn};

use crate::types::{BridgeRequest, BridgeResponse};

struct PendingEntry {
    tx: oneshot::Sender<BridgeResponse>,
    cancel: Arc<tokio::sync::Notify>,
}

/// Manages the single WebSocket connection from the Figma plugin
/// and matches responses to pending requests via request IDs.
pub struct Bridge {
    sink: Arc<Mutex<Option<futures_util::stream::SplitSink<WebSocket, Message>>>>,
    pending: Arc<DashMap<String, PendingEntry>>,
    counter: AtomicU64,
}

impl Default for Bridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Bridge {
    pub fn new() -> Self {
        Self {
            sink: Arc::new(Mutex::new(None)),
            pending: Arc::new(DashMap::new()),
            counter: AtomicU64::new(0),
        }
    }

    /// Accept a new WebSocket connection, replacing any existing one.
    pub async fn handle_connection(&self, ws: WebSocket) {
        let (sink, mut stream) = ws.split();

        let old = {
            let mut guard = self.sink.lock().await;
            guard.replace(sink)
        };
        if let Some(mut old_sink) = old {
            let _ = old_sink.close().await;
            info!("plugin connected (replaced previous connection)");
        } else {
            info!("plugin connected");
        }

        let pending = self.pending.clone();

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    handle_bridge_message(&text, &pending, "text");
                }
                Ok(Message::Binary(data)) => {
                    let text = String::from_utf8_lossy(&data);
                    handle_bridge_message(&text, &pending, "binary");
                }
                Ok(Message::Close(_)) => {
                    info!("plugin disconnected (close frame)");
                    break;
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                Err(e) => {
                    warn!("ws read error: {}", e);
                    break;
                }
            }
        }

        {
            let mut guard = self.sink.lock().await;
            guard.take();
        }
        self.fail_all_pending("plugin disconnected");
        info!("plugin disconnected");
    }

    /// Send a request to the plugin and wait for the response.
    pub async fn send(
        &self,
        msg_type: &str,
        node_ids: Vec<String>,
        params: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<BridgeResponse, String> {
        let request_id = self.next_id();
        let req = BridgeRequest {
            msg_type: msg_type.to_string(),
            request_id: request_id.clone(),
            node_ids,
            params,
        };

        let (tx, rx) = oneshot::channel();
        let cancel = Arc::new(tokio::sync::Notify::new());

        let entry = PendingEntry {
            tx,
            cancel: cancel.clone(),
        };

        self.pending.insert(request_id.clone(), entry);

        let json = serde_json::to_string(&req).map_err(|e| format!("serialize: {}", e))?;

        {
            let mut guard = self.sink.lock().await;
            let sink = match guard.as_mut() {
                Some(s) => s,
                None => {
                    self.pending.remove(&request_id);
                    return Err("plugin not connected".into());
                }
            };
            if sink.send(Message::Text(json.into())).await.is_err() {
                self.pending.remove(&request_id);
                return Err("send: WebSocket write error".into());
            }
        }

        let timeout_dur = bridge_timeout(msg_type);
        let req_id = request_id.clone();

        tokio::select! {
            result = rx => {
                match result {
                    Ok(resp) => Ok(resp),
                    Err(_) => Err("request timed out".into()),
                }
            }
            _ = tokio::time::sleep(timeout_dur) => {
                self.pending.remove(&req_id);
                Err("request timed out".into())
            }
        }
    }

    /// Close the bridge, rejecting all pending requests.
    pub async fn close(&self) {
        let sink = {
            let mut guard = self.sink.lock().await;
            guard.take()
        };
        if let Some(mut sink) = sink {
            let _ = sink.close().await;
        }
        self.fail_all_pending("bridge closed");
    }

    /// Reports whether the plugin is currently connected.
    pub fn is_connected(&self) -> bool {
        self.sink.try_lock().is_ok_and(|guard| guard.is_some())
    }

    fn fail_all_pending(&self, reason: &str) {
        let keys: Vec<String> = self.pending.iter().map(|e| e.key().clone()).collect();
        for key in keys {
            if let Some((_, entry)) = self.pending.remove(&key) {
                let _ = entry.tx.send(BridgeResponse {
                    error: reason.to_string(),
                    ..Default::default()
                });
            }
        }
    }

    fn next_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        let now = time_of_day();
        format!("req-{:02}{:02}{:02}-{}", now.0, now.1, now.2, n)
    }
}

/// Parse a bridge response message and dispatch it to the matching pending request.
/// Extracted to unify Text and Binary message handling.
fn handle_bridge_message(text: &str, pending: &DashMap<String, PendingEntry>, source: &str) {
    let resp: BridgeResponse = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(e) => {
            warn!("failed to parse bridge response ({}): {}", source, e);
            return;
        }
    };

    if resp.progress > 0 && !resp.request_id.is_empty() {
        debug!("progress {}: {}% {}", resp.request_id, resp.progress, resp.message);
        if let Some(entry) = pending.get(&resp.request_id) {
            entry.cancel.notify_one();
        }
        return;
    }

    if resp.request_id.is_empty() {
        warn!("received message with empty requestID — ignored");
        return;
    }

    if let Some((_, entry)) = pending.remove(&resp.request_id) {
        entry.cancel.notify_one();
        let _ = entry.tx.send(resp);
    } else {
        debug!("← {} received but no pending entry", resp.request_id);
    }
}

/// Bridge timeout for a given message type.
/// `get_design_context` defaults to 60s; all others default to 30s.
/// Overridable via `FIGMA_MCP_TIMEOUT_DESIGN_CONTEXT` and `FIGMA_MCP_TIMEOUT` env vars (seconds).
fn bridge_timeout(msg_type: &str) -> Duration {
    let (default, env_key) = if msg_type == "get_design_context" {
        (60u64, "FIGMA_MCP_TIMEOUT_DESIGN_CONTEXT")
    } else {
        (30u64, "FIGMA_MCP_TIMEOUT")
    };
    Duration::from_secs(
        std::env::var(env_key)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(default),
    )
}

fn time_of_day() -> (u32, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let day_secs = secs % 86400;
    let hour = (day_secs / 3600) as u32;
    let min = ((day_secs % 3600) / 60) as u32;
    let sec = (day_secs % 60) as u32;
    (hour, min, sec)
}
