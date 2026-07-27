use std::net::TcpListener;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::{error, info};

use crate::bridge::Bridge;
use crate::schema::validate_rpc;
use crate::types::{RPCRequest, RPCResponse};

/// Leader owns the WebSocket bridge to the Figma plugin and exposes
/// HTTP endpoints for health checks and follower RPC proxying.
pub struct Leader {
    ip: String,
    port: u16,
    bridge: Arc<Bridge>,
    version: String,
    shutdown: Option<Arc<tokio::sync::Notify>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Leader {
    pub fn new(ip: &str, port: u16, version: &str) -> Self {
        Self {
            ip: ip.to_string(),
            port,
            bridge: Arc::new(Bridge::new()),
            version: version.to_string(),
            shutdown: None,
            server_handle: None,
        }
    }

    pub fn bridge(&self) -> &Arc<Bridge> {
        &self.bridge
    }

    /// Bind the port and begin serving. Returns Err if port is in use.
    pub fn start(&mut self) -> Result<(), String> {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = TcpListener::bind(&addr).map_err(|e| e.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|e| e.to_string())?;

        let bridge = self.bridge.clone();
        let version = self.version.clone();

        let app = Router::new()
            .route("/ping", get(handle_ping))
            .route("/rpc", post(handle_rpc))
            .route("/ws", get(handle_ws))
            .with_state(AppState { bridge, version });

        let notify = Arc::new(tokio::sync::Notify::new());
        let notify_clone = notify.clone();
        let ip = self.ip.clone();
        let port = self.port;

        let handle = tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::from_std(listener) {
                Ok(l) => l,
                Err(e) => {
                    error!("failed to convert TcpListener: {}", e);
                    return;
                }
            };
            info!("listening on {}:{}", ip, port);

            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    notify_clone.notified().await;
                })
                .await
                .ok();
        });

        self.shutdown = Some(notify);
        self.server_handle = Some(handle);
        Ok(())
    }

    /// Shut down the HTTP server and close the bridge.
    pub async fn stop(&mut self) {
        if let Some(notify) = &self.shutdown {
            notify.notify_one();
        }
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }
        self.bridge.close().await;
    }
}

#[derive(Clone)]
struct AppState {
    bridge: Arc<Bridge>,
    version: String,
}

async fn handle_ping(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": state.version,
    }))
}

async fn handle_ws(
    ws: axum::extract::WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        state.bridge.handle_connection(socket).await;
    })
}

async fn handle_rpc(
    State(state): State<AppState>,
    Json(req): Json<RPCRequest>,
) -> Result<Json<RPCResponse>, StatusCode> {
    let params = req.params.clone().unwrap_or_default();

    if let Err(e) = validate_rpc(&req.tool, &req.node_ids, &params) {
        info!("rpc {} validation error: {}", req.tool, e);
        return Ok(Json(RPCResponse {
            error: e,
            ..Default::default()
        }));
    }

    match state
        .bridge
        .send(&req.tool, req.node_ids, req.params)
        .await
    {
        Ok(resp) => {
            if !resp.error.is_empty() {
                info!("rpc {} plugin error: {}", req.tool, resp.error);
                Ok(Json(RPCResponse {
                    error: resp.error,
                    ..Default::default()
                }))
            } else {
                Ok(Json(RPCResponse {
                    data: resp.data,
                    ..Default::default()
                }))
            }
        }
        Err(e) => {
            error!("rpc {} bridge error: {}", req.tool, e);
            Ok(Json(RPCResponse {
                error: e,
                ..Default::default()
            }))
        }
    }
}
