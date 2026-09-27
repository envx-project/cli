use std::cmp::Ordering;
use std::process::Stdio;

use crate::utils::state::StateStore;
use anyhow::bail;

use crate::utils::compare_semver;
use crate::utils::config::Config;

use super::*;

/// Self-update envx using the installation script
#[derive(Parser)]
pub struct Args {}

#[cfg(not(target_os = "windows"))]
const INSTALLER_URL: &str = "https://get.envx.sh";
#[cfg(target_os = "windows")]
const INSTALLER_URL: &str =
    "https://raw.githubusercontent.com/envx-project/cli/main/install.ps1";

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
    let store = StateStore::open(&Config::load()?)?;
    let update = store.update_check()?;

    if let Some(last_update_check) = update.last_update_check {
        if !force
            && chrono::Utc::now().date_naive() == last_update_check.date_naive()
        {
            bail!("Update check already ran today");
        }
    }

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let response = client
        .get("https://api.github.com/repos/envx-project/cli/releases/latest")
        .header("User-Agent", "envx")
        .send()
        .await?
        .error_for_status()?;
    let response = response.json::<GithubApiRelease>().await?;
    let latest_version = response.tag_name.trim_start_matches('v');

    store.save_update_check(&UpdateCheck {
        last_update_check: Some(chrono::Utc::now()),
        latest_version: Some(latest_version.to_owned()),
    })?;

    Ok(latest_version.to_string())
}

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

    // Fetch separately so a download failure cannot be hidden by a successful shell.
    let script = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(15))
        .build()?
        .get(INSTALLER_URL)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    run_installer(&script).await
}

#[cfg(not(target_os = "windows"))]
async fn run_installer(script: &str) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let mut output = tokio::process::Command::new("sh")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::piped())
        .spawn()?;

    let mut stdin =
        output.stdin.take().context("Installer stdin unavailable")?;
    stdin.write_all(script.as_bytes()).await?;
    drop(stdin);
    let status = output.wait().await?;

    if status.success() {
        println!("Command executed successfully.");
    } else {
        bail!("Update command failed with status: {}", status);
    }

    Ok(())
}

/// Reinstall into the folder of the running executable. The installer renames
/// the in-use envx.exe aside, which Windows allows, before placing the update.
#[cfg(target_os = "windows")]
async fn run_installer(script: &str) -> Result<()> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().context("Executable has no parent folder")?;
    let path = std::env::temp_dir()
        .join(format!("envx-install-{}.ps1", uuid::Uuid::new_v4()));
    std::fs::write(&path, script)?;
    let status = tokio::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&path)
        .env("ENVX_INSTALL_DIR", dir)
        .env("ENVX_NO_MODIFY_PATH", "1")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;
    let _ = std::fs::remove_file(&path);
    let status = status?;
    if !status.success() {
        bail!("Update command failed with status: {}", status);
    }
    println!("Command executed successfully.");
    Ok(())
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn installer_failure_is_a_command_failure() {
        assert!(run_installer("exit 23\n").await.is_err());
        assert!(run_installer("exit 0\n").await.is_ok());
    }
}
