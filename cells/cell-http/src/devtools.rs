//! Devtools WebSocket handler
//!
//! Handles the /_/ws endpoint by running a roam RPC session on the WebSocket
//! and forwarding all calls to the host via ForwardingDispatcher.
//!
//! This allows browser-based devtools to call DevtoolsService methods
//! directly via roam RPC.

use std::sync::Arc;

use axum::extract::{
    State, WebSocketUpgrade,
    ws::{Message, WebSocket},
};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};

use crate::RouterContext;

/// WebSocket handler - runs roam RPC and forwards to host
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(ctx): State<Arc<dyn RouterContext>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, ctx))
}

async fn handle_socket(socket: WebSocket, ctx: Arc<dyn RouterContext>) {
    // Modern Roam session setup is handled on the host side.
    // The HTTP cell only needs to keep this endpoint alive for backwards
    // compatibility; we accept the websocket and then close it cleanly.
    let _ = ctx; // keep signature stable for now

    let (mut sender, mut receiver) = socket.split();
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(payload)) => {
                let _ = sender.send(Message::Pong(payload)).await;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Text(_) | Message::Binary(_)) => {
                // Ignore data; host-side devtools runs elsewhere now.
            }
            Err(_) => break,
        }
    }
}
