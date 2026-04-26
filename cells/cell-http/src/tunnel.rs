//! TCP tunnel implementation for the cell.
//!
//! Implements the `TcpTunnel` service that the host calls to open tunnels.
//! Each tunnel serves HTTP directly over vox channels.

use std::io;
use std::sync::Arc;

use cell_http_proto::{TcpTunnel, Tunnel};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

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
    async fn open(&self, tunnel: Tunnel) {
        let service = self.app.clone();
        let _ctx = self.ctx.clone();

        tokio::spawn(async move {
            if let Err(error) = serve_tunnel(service, tunnel).await {
                tracing::warn!(error = %error, "HTTP tunnel error");
            }
        });
    }
}

async fn serve_tunnel(service: axum::Router, tunnel: Tunnel) -> io::Result<()> {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let (client_reader, client_writer) = tokio::io::split(client);
    let Tunnel { tx, rx } = tunnel;

    let tunnel_to_http =
        tokio::spawn(async move { pump_tunnel_to_writer(rx, client_writer).await });
    let http_to_tunnel =
        tokio::spawn(async move { pump_reader_to_tunnel(client_reader, tx).await });

    let http_result = hyper::server::conn::http1::Builder::new()
        .serve_connection(
            hyper_util::rt::TokioIo::new(server),
            hyper_util::service::TowerToHyperService::new(service),
        )
        .with_upgrades()
        .await
        .map_err(io::Error::other);

    let tunnel_to_http = tunnel_to_http
        .await
        .map_err(|error| io::Error::other(format!("tunnel->http task failed: {error}")))?;
    let http_to_tunnel = http_to_tunnel
        .await
        .map_err(|error| io::Error::other(format!("http->tunnel task failed: {error}")))?;

    http_result?;
    tunnel_to_http?;
    http_to_tunnel?;
    Ok(())
}

async fn pump_tunnel_to_writer<W>(mut rx: vox::Rx<Vec<u8>>, mut writer: W) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    loop {
        match rx.recv().await {
            Ok(Some(bytes)) => {
                let bytes = take_owned(bytes);
                writer.write_all(&bytes).await?;
            }
            Ok(None) => {
                writer.shutdown().await?;
                return Ok(());
            }
            Err(error) => {
                return Err(io::Error::other(format!(
                    "failed to receive tunnel bytes: {error:?}"
                )));
            }
        }
    }
}

async fn pump_reader_to_tunnel<R>(mut reader: R, tx: vox::Tx<Vec<u8>>) -> io::Result<()>
where
    R: AsyncRead + Unpin,
{
    let tx = tx;
    let mut buf = vec![0_u8; 16 * 1024];

    loop {
        let read = reader.read(&mut buf).await?;
        if read == 0 {
            tx.close(vox::Metadata::default())
                .await
                .map_err(|error| io::Error::other(format!("failed to close tunnel: {error:?}")))?;
            return Ok(());
        }

        tx.send(buf[..read].to_vec())
            .await
            .map_err(|error| io::Error::other(format!("failed to send tunnel bytes: {error:?}")))?;
    }
}

fn take_owned<T: 'static>(value: vox::SelfRef<T>) -> T {
    match value.try_map(|owned| Err::<(), _>(owned)) {
        Ok(_) => unreachable!("take_owned always returns the owned value"),
        Err(owned) => owned,
    }
}
