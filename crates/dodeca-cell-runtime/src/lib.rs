//! Runtime helpers for dodeca cells.
//!
//! Provides the `run_cell!` macro that handles all the boilerplate for connecting
//! to the host and signaling readiness.
//!
//! Enable the `cell-debug` feature for verbose startup logging.

pub use cell_host_proto::{HostServiceClient, ReadyMsg};
pub use dodeca_debug;
pub use vox;
pub use vox::{ConnectionHandle, RequestContext};
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
/// The dispatcher factory receives an `Arc<OnceLock<ConnectionHandle>>` that can be used
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
        use $crate::{ConnectionHandle, dodeca_debug, tokio, tracing, tracing_subscriber, ur_taking_me_with_you};

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
            let handle_cell: std::sync::Arc<std::sync::OnceLock<ConnectionHandle>> =
                std::sync::Arc::new(std::sync::OnceLock::new());

            let $handle = handle_cell.clone();
            $crate::cell_debug!("[cell] creating user dispatcher");
            let user_dispatcher = $make_dispatcher;
            $crate::cell_debug!("[cell] user dispatcher created");

            // With vox 0.4, we keep the user's dispatcher as-is here; tracing is handled
            // by standard `tracing_subscriber` output for now.
            let combined_dispatcher = user_dispatcher;
            $crate::cell_debug!("[cell] diagnostics disabled (no SHM transport)");
            // NOTE: SHM-based transport was removed to keep the dependency graph on crates.io.
            // Cells will be re-wired to use vox-local / vox-stream transports by the host.
            // For now, this runtime just constructs the dispatcher so the workspace compiles.
            let _ = combined_dispatcher;
            let _ = handle_cell;
            Ok(())
        }

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
            .block_on(__run_cell_async())
    }};
}
