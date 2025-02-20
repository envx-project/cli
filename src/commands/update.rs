use std::cmp::Ordering;

use crate::utils::{compare_semver, config::Config};

use super::*;

/// If your key is not in the database, use this command to upload it
#[derive(Parser)]
pub struct Args {}

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
        // println!(
        //     "Run `{}` to update\n",
        //     "curl -fsSL https://get.envx.sh | sh".green()
        // );
    }

    let mut output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg("curl -fsSL https://get.envx.sh | sh")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit())
        .spawn()?;

    let status = output.wait().await?;

    if status.success() {
        println!("Command executed successfully.");
    } else {
        eprintln!("Command failed with status: {:?}", status);
    }

    Ok(())
}
