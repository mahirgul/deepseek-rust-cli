use std::sync::OnceLock;

use anyhow::Result;
use self_update::backends::github::Update;

use crate::version::VERSION;

static LATEST_VERSION: OnceLock<Option<String>> = OnceLock::new();

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
    let status = Update::configure()
        .repo_owner("mahirgul")
        .repo_name("deepseek-rust-cli")
        .bin_name("deepseek-rust-cli")
        .show_download_progress(true)
        .current_version(VERSION)
        .no_confirm(false)
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
