use std::fs;

use super::*;
use crate::utils::config::get_config;
use anyhow::Result;

/// Initialize a new env-store
#[derive(Debug, Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    get_config()?.write(false)?;

    println!("Initialized new envcli store");

    if !std::path::Path::new(".envcli.vault").exists() {
        println!("Creating new empty vault");
        fs::write(".envcli.vault", "[]")
            .context("Failed to create new vault file, try running `envcli init`")?;
    }

    Ok(())
}
