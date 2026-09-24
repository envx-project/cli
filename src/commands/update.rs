use std::cmp::Ordering;
use std::process::Stdio;

use crate::utils::state::StateStore;
use anyhow::bail;

use crate::utils::compare_semver;
use crate::utils::config::Config;

use super::*;

/// Attempt to self-update envx using the installation script. Fails on Windows.
#[derive(Parser)]
pub struct Args {}

#[cfg(target_os = "windows")]
pub async fn command(_args: Args, _config: &mut Config) -> Result<()> {
    use anyhow::bail;

    eprintln!("Self-update is not supported on Windows");
    eprintln!("Read the installation instructions at https://github.com/envx-project/cli/blob/main/windows-installation.md");

    Ok(())
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct UpdateCheck {
    pub last_update_check: Option<chrono::DateTime<chrono::Utc>>,
    pub latest_version: Option<String>,
}
#[derive(serde::Deserialize)]
struct GithubApiRelease {
    tag_name: String,
}

pub async fn check_update(force: bool) -> anyhow::Result<String> {
    let store = StateStore::open(&Config::get())?;
    let update = store.update_check()?;

    if let Some(last_update_check) = update.last_update_check {
        if !force
            && chrono::Utc::now().date_naive() == last_update_check.date_naive()
        {
            bail!("Update check already ran today");
        }
    }

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/envx-project/cli/releases/latest")
        .header("User-Agent", "envx")
        .send()
        .await?;
    let response = response.json::<GithubApiRelease>().await?;
    let latest_version = response.tag_name.trim_start_matches('v');

    store.save_update_check(&UpdateCheck {
        last_update_check: Some(chrono::Utc::now()),
        latest_version: Some(latest_version.to_owned()),
    })?;

    Ok(latest_version.to_string())
}

#[cfg(not(target_os = "windows"))]
pub async fn command(_args: Args, _config: &mut Config) -> Result<()> {
    let latest_version = check_update(true).await?;

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
