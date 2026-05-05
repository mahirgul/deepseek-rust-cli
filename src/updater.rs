use anyhow::Result;
use self_update::backends::github::Update;

use crate::version::VERSION;

/// Detect platform in the short format used by our release assets (e.g. `linux-x86_64`)
fn get_short_target() -> String {
    let os = std::env::consts::OS; // "linux", "macos", "windows"
    let arch = std::env::consts::ARCH; // "x86_64", "aarch64"

    // Normalize arch names
    let arch_short = match arch {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => arch,
    };

    format!("{}-{}", os, arch_short)
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
