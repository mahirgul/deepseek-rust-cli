use crate::version::VERSION;
use anyhow::Result;
use self_update::backends::github::Update;
use std::sync::OnceLock;

static LATEST_VERSION: OnceLock<Option<String>> = OnceLock::new();

/// Silently check for updates in the background
pub fn check_for_updates_background() {
    std::thread::spawn(|| {
        let _ = check_latest_version();
    });
}

/// Get the latest version string if it has been checked
pub fn get_latest_available_version() -> Option<String> {
    LATEST_VERSION.get().cloned().flatten()
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
pub fn run_update() -> Result<()> {
    println!("Checking for updates...");

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
        println!("Successfully updated to version {}!", status.version());
        println!("Please restart the application.");
        std::process::exit(0);
    } else {
        println!("You are already using the latest version ({}).", VERSION);
    }

    Ok(())
}
