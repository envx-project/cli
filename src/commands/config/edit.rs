use super::*;
use crate::utils::config::{get_config_file_path, Config};
use crate::utils::prompt::{prompt_confirm_with_default, require_interactive};
use anyhow::bail;
use chrono::Utc;
use std::fs;
use std::process::Command;

/// Open the config file in $EDITOR and validate on save
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, config: &mut Config) -> Result<()> {
    require_interactive(
        "`envx config edit` opens an editor and needs a TTY.",
        "Use `envx config set <path> <value>` to edit non-interactively.",
    )?;

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let config_path =
        get_config_file_path().context("Failed to get config path")?;
    let original = fs::read_to_string(&config_path)
        .context("Failed to read config file")?;

    let mut temp_path = config_path.clone();
    let nanos = Utc::now().timestamp_nanos_opt().unwrap();
    let pid = std::process::id();
    temp_path.set_extension(format!("edit.{}-{}.json", pid, nanos));
    fs::write(&temp_path, &original)
        .context("Failed to write temp edit buffer")?;

    loop {
        let status = Command::new(&editor)
            .arg(&temp_path)
            .status()
            .with_context(|| format!("Failed to spawn editor '{}'", editor))?;

        if !status.success() {
            let _ = fs::remove_file(&temp_path);
            bail!("Editor exited with status {}", status);
        }

        let edited = fs::read_to_string(&temp_path)
            .context("Failed to read edited config")?;

        if edited == original {
            let _ = fs::remove_file(&temp_path);
            println!("No changes.");
            return Ok(());
        }

        match serde_json::from_str::<Config>(&edited) {
            Ok(new_config) => {
                *config = new_config;
                let _ = fs::remove_file(&temp_path);
                println!("{}", "Config updated.".green());
                return Ok(());
            }
            Err(e) => {
                eprintln!("{}: {}", "Invalid config".red(), e);
                let retry =
                    prompt_confirm_with_default("Re-open the editor?", true)?;
                if !retry {
                    let _ = fs::remove_file(&temp_path);
                    bail!("Aborted. No changes written.");
                }
            }
        }
    }
}
