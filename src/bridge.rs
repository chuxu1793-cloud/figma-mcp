use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{oneshot, Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::types::{BridgeRequest, BridgeResponse};

struct PendingEntry {
    tx: oneshot::Sender<BridgeResponse>,
    cancel: Arc<tokio::sync::Notify>,
}

/// Manages the single WebSocket connection from the Figma plugin
/// and matches responses to pending requests via request IDs.
pub struct Bridge {
    sink: RwLock<Option<Arc<Mutex<futures_util::stream::SplitSink<WebSocket, Message>>>>>,
    pending: Arc<DashMap<String, PendingEntry>>,
    counter: AtomicU64,
}

impl Bridge {
    pub fn new() -> Self {
        Self {
            sink: RwLock::new(None),
            pending: Arc::new(DashMap::new()),
            counter: AtomicU64::new(0),
        }
    }

    /// Accept a new WebSocket connection, replacing any existing one.
    pub async fn handle_connection(&self, ws: WebSocket) {
        let (sink, mut stream) = ws.split();

        let sink = Arc::new(Mutex::new(sink));
        let old = self.sink.write().await.replace(sink.clone());
        if let Some(old_sink) = old {
            if let Ok(mut s) = old_sink.try_lock() {
                let _ = s.close().await;
            }
            info!("plugin connected (replaced previous connection)");
        } else {
            info!("plugin connected");
        }

        let pending = self.pending.clone();

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let resp: BridgeResponse = match serde_json::from_str(&text) {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("failed to parse bridge response: {}", e);
                            continue;
                        }
                    };

                    if resp.progress > 0 && !resp.request_id.is_empty() {
                        debug!("progress {}: {}% {}", resp.request_id, resp.progress, resp.message);
                        if let Some(entry) = pending.get(&resp.request_id) {
                            entry.cancel.notify_one();
                        }
                        continue;
                    }

                    if resp.request_id.is_empty() {
                        warn!("received message with empty requestID — ignored");
                        continue;
                    }

                    if let Some((_, entry)) = pending.remove(&resp.request_id) {
                        entry.cancel.notify_one();
                        let _ = entry.tx.send(resp);
                    } else {
                        debug!("← {} received but no pending entry", resp.request_id);
                    }
                }
                Ok(Message::Binary(data)) => {
                    let text = String::from_utf8_lossy(&data);
                    let resp: BridgeResponse = match serde_json::from_str(&text) {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("failed to parse bridge response (binary): {}", e);
                            continue;
                        }
                    };

                    if resp.progress > 0 && !resp.request_id.is_empty() {
                        if let Some(entry) = pending.get(&resp.request_id) {
                            entry.cancel.notify_one();
                        }
                        continue;
                    }

                    if resp.request_id.is_empty() {
                        continue;
                    }

                    if let Some((_, entry)) = pending.remove(&resp.request_id) {
                        entry.cancel.notify_one();
                        let _ = entry.tx.send(resp);
                    }
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

        self.sink.write().await.take();
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
        let sink_guard = self.sink.read().await;
        let sink = sink_guard.as_ref().ok_or("plugin not connected")?.clone();
        drop(sink_guard);

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
            let mut sink = sink.lock().await;
            if sink.send(Message::Text(json.into())).await.is_err() {
                self.pending.remove(&request_id);
                return Err("send: WebSocket write error".into());
            }
        }

        let timeout_dur = if msg_type == "get_document" {
            Duration::from_secs(60)
        } else {
            Duration::from_secs(30)
        };

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
        let sink = self.sink.write().await.take();
        if let Some(sink) = sink {
            if let Ok(mut s) = sink.try_lock() {
                let _ = s.close().await;
            }
        }
        self.fail_all_pending("bridge closed");
    }

    /// Reports whether the plugin is currently connected.
    pub fn is_connected(&self) -> bool {
        self.sink.try_read().map_or(false, |guard| guard.is_some())
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
        let now = chrono_like_now();
        format!("req-{:02}{:02}{:02}-{}", now.0, now.1, now.2, n)
    }
}

fn chrono_like_now() -> (u32, u32, u32) {
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
