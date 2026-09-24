use super::*;
use crate::utils::{
    config::{Config, Project},
    state::StateStore,
};
use anyhow::bail;
use std::{fs, io::Write, path::Path};

/// Import the old ~/.config/envcli profile, preserving its source files.
#[derive(Parser)]
pub struct Args {
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn copy_preserving(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        if fs::read(source)? != fs::read(destination)? {
            bail!(
                "Destination already contains different data: {}",
                destination.display()
            );
        }
        return Ok(());
    }
    let temporary = destination.with_extension(format!(
        "import.{}.{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let result = (|| -> Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(&fs::read(source)?)?;
        file.sync_all()?;
        fs::hard_link(&temporary, destination)?;
        Ok(())
    })();
    let _ = fs::remove_file(temporary);
    result?;
    Ok(())
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let home = crate::utils::paths::home_dir()
        .context("Failed to get home directory")?;
    let old = home.join(".config/envcli");
    let new = home.join(".config/envx");
    if !old.join("config.json").exists() {
        println!("No legacy envcli config found; SQLite state migrates automatically.");
        return Ok(());
    }
    // Never overwrite an established profile. Retrying a completed import is safe.
    let original = fs::read(old.join("config.json"))?;
    let already_copied = fs::read(new.join("config.json"))? == original;
    if !already_copied
        && (config.primary_key.is_some() || !config.projects.is_empty())
    {
        bail!("An envx profile already exists; legacy files were preserved. Use a separate HOME to import it.");
    }
    Config::decode(std::str::from_utf8(&original)?)?;
    let value: serde_json::Value = serde_json::from_slice(&original)
        .context("Invalid legacy config; source preserved")?;
    let projects: Vec<Project> = serde_json::from_value(
        value
            .get("projects")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    )?;
    if old.join("keys").exists() {
        for entry in walkdir::WalkDir::new(old.join("keys")) {
            let entry = entry?;
            let destination = new.join(entry.path().strip_prefix(&old)?);
            if entry.file_type().is_symlink() {
                bail!(
                    "Refusing legacy key symlink: {}",
                    entry.path().display()
                );
            }
            if entry.file_type().is_dir() {
                fs::create_dir_all(destination)?;
            } else {
                copy_preserving(entry.path(), &destination)?;
            }
        }
    }
    let staging =
        new.join(format!("config.migrate.{}.json", std::process::id()));
    copy_preserving(&old.join("config.json"), &staging)?;
    fs::rename(staging, new.join("config.json"))?;
    let migrated = Config::load()?;
    StateStore::open(&migrated)?.import_projects("envcli-v1", &projects)?;
    *config = Config::load()?;
    if args.verbose {
        println!("Original profile preserved at {}", old.display());
    }
    println!(
        "Migration complete. Original config and keys have been preserved."
    );
    Ok(())
}
