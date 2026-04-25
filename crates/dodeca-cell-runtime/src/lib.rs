//! Runtime helpers for dodeca cells.
//!
//! Provides the `run_cell!` macro that handles all the boilerplate for connecting
//! to the host and signaling readiness.
//!
//! Enable the `cell-debug` feature for verbose startup logging.

pub use cell_host_proto::{HostServiceClient, ReadyMsg};
pub use dodeca_debug;
pub use vox;
pub use vox::{Caller, ConnectionHandle, RequestContext};
pub use tokio;
pub use tracing;
pub use tracing_subscriber;
pub use ur_taking_me_with_you;

/// Debug print macro that only prints when cell-debug feature is enabled
#[macro_export]
#[cfg(feature = "cell-debug")]
macro_rules! cell_debug {
    ($($arg:tt)*) => {
        eprintln!($($arg)*)
    };
}

/// Debug print macro that compiles to nothing when cell-debug feature is disabled
#[macro_export]
#[cfg(not(feature = "cell-debug"))]
macro_rules! cell_debug {
    ($($arg:tt)*) => {};
}

/// Run a cell with the given name and dispatcher factory.
///
/// The dispatcher factory receives an `Arc<OnceLock<Caller>>` that can be used
/// to create clients for calling back to the host. Cells that don't need callbacks
/// can ignore this parameter.
///
/// # Examples
///
/// Cell that doesn't need callbacks:
/// ```ignore
/// use dodeca_cell_runtime::run_cell;
/// use cell_image_proto::{ImageProcessorDispatcher, ImageProcessorImpl};
///
/// fn main() {
///     run_cell!("image", |_handle| {
///         ImageProcessorDispatcher::new(ImageProcessorImpl)
///     });
/// }
/// ```
///
/// Cell with callbacks to host:
/// ```ignore
/// use dodeca_cell_runtime::run_cell;
///
/// fn main() {
///     run_cell!("html", |handle| {
///         let processor = HtmlProcessorImpl::new(handle);
///         HtmlProcessorDispatcher::new(processor)
///     });
/// }
/// ```
#[macro_export]
macro_rules! run_cell {
    ($cell_name:expr, |$handle:ident| $make_dispatcher:expr) => {{
        use tracing_subscriber::prelude::*;
        use $crate::{Caller, dodeca_debug, tokio, tracing, tracing_subscriber, ur_taking_me_with_you};

        $crate::cell_debug!(
            "[cell-{}] starting (pid={})",
            $cell_name,
            std::process::id()
        );

        // Install SIGUSR1 handler for diagnostics (must be done early, before async runtime)
        // We use a leaked static string since install_sigusr1_handler expects &'static str
        let cell_name_static: &'static str =
            Box::leak(format!("cell-{}", $cell_name).into_boxed_str());
        dodeca_debug::install_sigusr1_handler(cell_name_static);

        // Register diagnostic callback (no-op without SHM transport)
        dodeca_debug::register_diagnostic(|| {});

        // Ensure this process dies when the parent dies (required for macOS pipe-based approach)
        ur_taking_me_with_you::die_with_parent();

        $crate::cell_debug!("[cell-{}] die_with_parent completed", $cell_name);

        async fn __run_cell_async() -> Result<(), Box<dyn std::error::Error>> {
            $crate::cell_debug!("[cell] async fn starting");

            // Initialize cell-side tracing (stderr passthrough in sandbox-friendly mode)
            // If TRACING_PASSTHROUGH is set, log to stderr; otherwise use a standard fmt layer.
            let use_passthrough = std::env::var("TRACING_PASSTHROUGH").is_ok();
            $crate::cell_debug!("[cell] use_passthrough={}", use_passthrough);
            use $crate::tracing_subscriber::EnvFilter;
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_ansi(false)
                .with_target(true)
                .with_env_filter(EnvFilter::from_default_env())
                .init();
            $crate::cell_debug!("[cell] tracing initialized");

            // Let user code create the dispatcher with access to handle
            // We use an Arc<OnceLock> pattern
            let handle_cell: std::sync::Arc<std::sync::OnceLock<Caller>> =
                std::sync::Arc::new(std::sync::OnceLock::new());

            let $handle = handle_cell.clone();
            $crate::cell_debug!("[cell] creating user dispatcher");
            let user_dispatcher = $make_dispatcher;
            $crate::cell_debug!("[cell] user dispatcher created");

            // Serve the cell over vox local transport.
            let endpoint = std::env::var("DODECA_CELL_ENDPOINT")
                .map_err(|_| "missing DODECA_CELL_ENDPOINT (expected local socket/pipe path)")?;
            let addr = format!("local://{endpoint}");

            let cell_name = format!("ddc-cell-{}", $cell_name);
            let pid = std::process::id();

            let host_acceptor = $crate::combined_acceptor(
                cell_name.clone(),
                pid,
                handle_cell.clone(),
                user_dispatcher,
            );

            $crate::cell_debug!("[cell] serving at {}", addr);
            $crate::vox::serve(addr, host_acceptor).await?;
            Ok(())
        }

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
            .block_on(__run_cell_async())
    }};
}

pub fn combined_acceptor<D>(
    cell_name: String,
    pid: u32,
    handle_cell: std::sync::Arc<std::sync::OnceLock<vox::Caller>>,
    dispatcher: D,
) -> impl vox_core::ConnectionAcceptor
where
    D: vox_types::Handler<vox::DriverReplySink> + Clone + Send + Sync + 'static,
{
    use vox_core::{ConnectionAcceptor, ConnectionRequest, PendingConnection};

    #[derive(Clone)]
    struct CellAcceptor<D> {
        cell_name: String,
        pid: u32,
        handle_cell: std::sync::Arc<std::sync::OnceLock<vox::Caller>>,
        dispatcher: D,
    }

    impl<D> ConnectionAcceptor for CellAcceptor<D>
    where
        D: vox_types::Handler<vox::DriverReplySink> + Clone + Send + Sync + 'static,
    {
        fn accept(
            &self,
            _request: &ConnectionRequest,
            connection: PendingConnection,
        ) -> Result<(), vox_types::Metadata<'static>> {
            let dispatcher = self.dispatcher.clone();
            let cell_name = self.cell_name.clone();
            let pid = self.pid;
            let handle_cell = self.handle_cell.clone();

            let handle = connection.into_handle();
            let mut driver = vox::Driver::new(handle, dispatcher);
            let caller = vox::Caller::new(driver.caller());
            let _ = handle_cell.set(caller.clone());

            tokio::spawn(async move {
                // Signal readiness to host (best-effort)
                let host = crate::HostServiceClient::new(caller.clone());
                let _ = host
                    .ready(crate::ReadyMsg {
                        peer_id: 0,
                        cell_name,
                        pid: Some(pid),
                        version: None,
                        features: vec![],
                    })
                    .await;

                driver.run().await;
            });

            Ok(())
        }
    }

    CellAcceptor {
        cell_name,
        pid,
        handle_cell,
        dispatcher,
    }
}
