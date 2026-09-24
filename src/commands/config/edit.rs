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
    write_private_buffer(&temp_path, &original)?;
    let _cleanup = EditBuffer(temp_path.clone());

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

        match Config::decode(&edited) {
            Ok(new_config) => {
                config.apply_edited(new_config, &original)?;
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

struct EditBuffer(std::path::PathBuf);
impl Drop for EditBuffer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn write_private_buffer(path: &std::path::Path, contents: &str) -> Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .context("Failed to create private edit buffer")?
        .write_all(contents.as_bytes())
        .context("Failed to write edit buffer")?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn edit_buffer_is_private_and_cannot_overwrite_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("edit.json");
        write_private_buffer(&path, "secret").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(write_private_buffer(&path, "replacement").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "secret");
        drop(EditBuffer(path.clone()));
        assert!(!path.exists());
    }
}
