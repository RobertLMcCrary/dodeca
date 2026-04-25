//! TCP tunnel implementation for the cell.
//!
//! Implements the `TcpTunnel` service that the host calls to open tunnels.
//! Each tunnel serves HTTP directly.
//!
//! NOTE: The old vox tunnel streaming helpers were removed upstream; this
//! module is currently a stub so the workspace can build and tests can run.

use std::sync::Arc;
use cell_http_proto::{TcpTunnel, Tunnel};

use crate::RouterContext;

/// Cell-side implementation of `TcpTunnel`.
///
/// Each `open()` call receives a tunnel from the host and serves HTTP on it.
#[derive(Clone)]
pub struct TcpTunnelImpl {
    #[allow(dead_code)]
    ctx: Arc<dyn RouterContext>,
    app: axum::Router,
}

impl TcpTunnelImpl {
    pub fn new(ctx: Arc<dyn RouterContext>, app: axum::Router) -> Self {
        Self { ctx, app }
    }
}

impl TcpTunnel for TcpTunnelImpl {
    async fn open(&self, _tunnel: Tunnel) {
        let _ = &self.app;
        let _ = &self.ctx;
        tracing::trace!("HTTP tunnel opened (stub)");
    }
}
