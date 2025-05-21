use std::cmp::Ordering;
use std::process::Stdio;

use crate::utils::{compare_semver, config::Config};

use super::*;

/// Attempt to self-update envx using the installation script. Fails on Windows.
#[derive(Parser)]
pub struct Args {}

#[cfg(target_os = "windows")]
pub async fn command(_args: Args) -> Result<()> {
    use anyhow::bail;

    eprintln!("Self-update is not supported on Windows");
    eprintln!("Read the installation instructions at https://github.com/envx-project/cli/blob/main/windows-installation.md");

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub async fn command(_args: Args) -> Result<()> {
    let mut config = Config::get()?;
    let result = config.check_update(true).await?;
    config.write()?;

    let latest_version = if let Some(latest_version) = result {
        latest_version
    } else {
        println!("No updates available");
        return Ok(());
    };

    if matches!(
        compare_semver(env!("CARGO_PKG_VERSION"), &latest_version),
        Ordering::Less
    ) {
        println!(
            "{}: v{} -> v{}",
            "Update available".green().bold(),
            env!("CARGO_PKG_VERSION").yellow(),
            latest_version.bright_yellow(),
        );
    } else {
        println!("You are already on or ahead of the latest version");
        println!("Current version: {}", env!("CARGO_PKG_VERSION"));
        println!("Latest version: {}", latest_version);
        return Ok(());
    }

    let mut output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://get.envx.sh | sh")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .spawn()?;

    let status = output.wait().await?;

    if status.success() {
        println!("Command executed successfully.");
    } else {
        eprintln!("Command failed with status: {:?}", status);
    }

    Ok(())
}
