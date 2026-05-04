use std::sync::OnceLock;

use anyhow::Result;
use self_update::backends::github::Update;

use crate::version::VERSION;

static LATEST_VERSION: OnceLock<Option<String>> = OnceLock::new();

/// Detect platform in the short format used by our release assets (e.g. `linux-x86_64`)
fn get_short_target() -> String {
    let os = std::env::consts::OS; // "linux", "macos", "windows"
    let arch = std::env::consts::ARCH; // "x86_64", "aarch64"

    // Map std::env::consts::OS to our short OS names
    let os_short = os;

    // Normalize arch names
    let arch_short = match arch {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => arch,
    };

    format!("{}-{}", os_short, arch_short)
}

/// Silently check for updates in the background
pub fn check_for_updates_background() {
    std::thread::spawn(|| {
        let _ = check_latest_version();
    });
}

/// Perform the actual check (can be slow)
fn check_latest_version() -> Result<String> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("mahirgul")
        .repo_name("deepseek-rust-cli")
        .build()?
        .fetch()?;

    if let Some(latest) = releases.first() {
        let version = latest.version.clone();
        let _ = LATEST_VERSION.set(Some(version.clone()));
        Ok(version)
    } else {
        let _ = LATEST_VERSION.set(None);
        Err(anyhow::anyhow!("No releases found"))
    }
}

/// Execute the update process
pub fn run_update() -> Result<String> {
    let target = get_short_target();

    let status = Update::configure()
        .repo_owner("mahirgul")
        .repo_name("deepseek-rust-cli")
        .bin_name("deepseek-rust-cli")
        .target(&target)
        .show_download_progress(false)
        .show_output(false)
        .current_version(VERSION)
        .no_confirm(true)
        .build()?
        .update()?;

    if status.updated() {
        Ok(format!(
            "✅ Successfully updated to version {}! Please restart the application.",
            status.version()
        ))
    } else {
        Ok(format!(
            "ℹ️ You are already using the latest version ({}).",
            VERSION
        ))
    }
}
