//! Cross-platform process sandboxing.
//!
//! This crate provides a unified API for running processes in a sandboxed environment
//! across different operating systems:
//!
//! - **Linux**: Uses [hakoniwa](https://docs.rs/hakoniwa) (namespaces, seccomp, landlock)
//! - **macOS**: Uses Seatbelt (sandbox-exec with SBPL profiles)
//!
//! # Example
//!
//! ```no_run
//! use dodeca_sandbox::{Sandbox, SandboxConfig};
//!
//! let config = SandboxConfig::new()
//!     .allow_read("/usr")
//!     .allow_read("/lib")
//!     .allow_read("/lib64")
//!     .allow_read_write("/tmp/my-project")
//!     .allow_execute("/usr/bin/cargo");
//!
//! let sandbox = Sandbox::new(config)?;
//! let output = sandbox
//!     .command("/usr/bin/cargo")
//!     .args(["build", "--release"])
//!     .current_dir("/tmp/my-project")
//!     .output()?;
//! # Ok::<(), dodeca_sandbox::Error>(())
//! ```

mod config;
mod error;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod unsupported;

pub use config::{PathAccess, SandboxConfig};
pub use error::Error;

#[cfg(target_os = "linux")]
pub use linux::{Command, ExitStatus, Output, Sandbox, Stdio};

#[cfg(target_os = "macos")]
pub use macos::{Command, ExitStatus, Output, Sandbox, Stdio};

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub use unsupported::{Command, ExitStatus, Output, Sandbox, Stdio};

/// Result type for sandbox operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_build() -> std::result::Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;

        // Create temp directory outside of /tmp to avoid conflict with
        // the sandbox's isolated tmpfs mount on Linux.
        //
        // In Nix builds, $HOME can be "/homeless-shelter" and may not exist; avoid relying on it.
        // Prefer a temp location outside the repo/workspace so Cargo won't pick up
        // workspace-level `.cargo/config.toml` from ancestor directories.
        #[cfg(target_os = "linux")]
        let base = std::path::PathBuf::from("/var/tmp").join("dodeca-sandbox-tests");
        #[cfg(not(target_os = "linux"))]
        let base = std::env::temp_dir().join("dodeca-sandbox-tests");
        std::fs::create_dir_all(&base)?;
        let temp_dir = tempfile::tempdir_in(&base)?;
        let project_dir = temp_dir.path();

        // Create a minimal Rust project
        std::fs::write(
            project_dir.join("Cargo.toml"),
            r#"[package]
name = "sandbox-test-project"
version = "0.1.0"
edition = "2021"

[dependencies]

# Ensure this temp project is treated as its own workspace even when
# generated under another Cargo workspace directory.
[workspace]
"#,
        )?;

        std::fs::create_dir(project_dir.join("src"))?;
        std::fs::write(
            project_dir.join("src/main.rs"),
            r#"fn main() {
    println!("Hello from sandboxed build!");
}
"#,
        )?;

        // Avoid touching the network in tests. This project has no deps.
        // SAFETY: test-local env var change in isolated process
        unsafe { std::env::set_var("CARGO_NET_OFFLINE", "true") };

        // Get paths from environment
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| base.to_string_lossy().into());
        let rustup_home =
            std::env::var("RUSTUP_HOME").unwrap_or_else(|_| format!("{home_dir}/.rustup"));
        let cargo_home =
            std::env::var("CARGO_HOME").unwrap_or_else(|_| format!("{home_dir}/.cargo"));

        // In Nix shells (and some CI), `rustup` may not be present. Fall back to PATH.
        let rustup_ok = Command::new("rustup").arg("--version").output().is_ok();

        // Get the active toolchain's bin directory
        let (toolchain_bin, cargo_path) = if rustup_ok {
            let rustc_path = Command::new("rustup")
                .args(["which", "rustc"])
                .output()
                .expect("failed to run rustup which rustc");
            let rustc_path = String::from_utf8_lossy(&rustc_path.stdout);
            let toolchain_bin = std::path::Path::new(rustc_path.trim())
                .parent()
                .expect("rustc path has no parent")
                .to_string_lossy()
                .to_string();

            let cargo_path_output = Command::new("rustup")
                .args(["which", "cargo"])
                .output()
                .expect("failed to run rustup which cargo");
            let cargo_path = String::from_utf8_lossy(&cargo_path_output.stdout)
                .trim()
                .to_string();
            (toolchain_bin, cargo_path)
        } else {
            let rustc_path = Command::new("which")
                .arg("rustc")
                .output()
                .expect("failed to run which rustc");
            let rustc_path = String::from_utf8_lossy(&rustc_path.stdout)
                .trim()
                .to_string();
            let toolchain_bin = std::path::Path::new(&rustc_path)
                .parent()
                .unwrap_or(std::path::Path::new("/usr/bin"))
                .to_string_lossy()
                .to_string();

            let cargo_path_output = Command::new("which")
                .arg("cargo")
                .output()
                .expect("failed to run which cargo");
            let cargo_path = String::from_utf8_lossy(&cargo_path_output.stdout)
                .trim()
                .to_string();

            (toolchain_bin, cargo_path)
        };

        // Configure sandbox for cargo build - platform specific paths
        let mut config = SandboxConfig::new()
            // Rust toolchain (read + execute)
            .allow_read_execute(&rustup_home)
            .allow_read_execute(&cargo_home)
            .allow_read_execute(&toolchain_bin)
            // Cargo itself may live outside CARGO_HOME (e.g. Nix profiles)
            .allow_execute(&cargo_path)
            // Project directory (read + write for build artifacts)
            .allow_full(project_dir)
            // Deny network (we've already fetched dependencies)
            .deny_network();

        // If cargo_path is a symlink, sandbox-exec may apply rules to the resolved target.
        if let Ok(resolved) = std::fs::canonicalize(&cargo_path) {
            config = config.allow_execute(resolved);
        }

        // Platform-specific paths
        #[cfg(target_os = "macos")]
        let (config, extra_env) = {
            // Get SDK path for linker
            let sdk_path = Command::new("xcrun")
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()
                .expect("failed to run xcrun --show-sdk-path");
            let sdk_path = String::from_utf8_lossy(&sdk_path.stdout).trim().to_string();

            // Get developer dir
            let developer_dir = Command::new("xcode-select")
                .args(["-p"])
                .output()
                .expect("failed to run xcode-select -p");
            let developer_dir = String::from_utf8_lossy(&developer_dir.stdout)
                .trim()
                .to_string();

            let config = config
                // System paths needed for compilation
                .allow_read_execute("/usr")
                .allow_read_execute("/bin")
                // Nix-provided toolchains and their dynamic libs
                .allow_read_execute("/nix/store")
                .allow_read_execute("/etc/profiles")
                // SSL/TLS configuration
                .allow_read("/private/etc")
                // xcode-select symlinks
                .allow_read("/private/var/select")
                // Homebrew (for linker, etc.)
                .allow_read_execute("/opt/homebrew")
                // Library paths
                .allow_read("/Library")
                // Xcode / developer tools (linker, SDK)
                .allow_read_execute("/Applications/Xcode.app")
                .allow_read_execute("/Applications/Xcode-beta.app")
                .allow_read_execute("/Library/Developer");

            let extra_env = vec![
                ("SDKROOT".to_string(), sdk_path),
                ("DEVELOPER_DIR".to_string(), developer_dir),
            ];
            (config, extra_env)
        };

        #[cfg(target_os = "linux")]
        let (config, extra_env) = {
            let config = config
                // System paths needed for compilation
                .allow_read_execute("/usr")
                .allow_read_execute("/bin")
                .allow_read_execute("/lib")
                .allow_read_execute("/lib64")
                // SSL/TLS configuration
                .allow_read("/etc")
                // Proc filesystem (needed by some tools)
                .allow_read("/proc");

            let extra_env: Vec<(String, String)> = vec![];
            (config, extra_env)
        };

        let sandbox = Sandbox::new(config)?;
        let mut cmd = sandbox
            .command(&cargo_path)
            .args(["build", "--release", "--offline"])
            .current_dir(project_dir)
            .env("RUSTUP_HOME", &rustup_home)
            .env("CARGO_HOME", &cargo_home)
            .env("HOME", &home_dir)
            .env("CARGO_NET_OFFLINE", "true")
            .env(
                "PATH",
                format!("{cargo_home}/bin:{toolchain_bin}:/usr/bin:/bin"),
            );

        // Add platform-specific env vars
        for (key, value) in extra_env {
            cmd = cmd.env(&key, &value);
        }

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "cargo build failed:\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Verify the binary was created
        let binary_path = project_dir.join("target/release/sandbox-test-project");
        assert!(
            binary_path.exists(),
            "binary was not created at {binary_path:?}"
        );
        Ok(())
    }
}
