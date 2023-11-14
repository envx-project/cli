use super::*;
use crate::utils::config::{get_config, Config};
use anyhow::Result;

/// Initialize a new env-store
#[derive(Debug, Parser)]
pub struct Args {
    /// Args to pass to the command
    #[clap(trailing_var_arg = true)]
    args: Vec<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config()?;

    let new_config = config.clone();

    new_config.write(false)?;

    std::fs::write(".envcli.vault", "[]")?;

    Ok(())
}
