use super::*;
use crate::{sdk::SDK, utils::config::get_config};
use anyhow::Context;

/// Unset (delete) an environment variable
#[derive(Parser)]
pub struct Args {
    /// Variable to delete
    #[clap(short, long)]
    variable: Option<String>,

    /// Key to use
    #[clap(short, long)]
    key: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config().context("Failed to get config")?;
    let key = config.get_key_or_default(args.key)?;

    let variable = match args.variable {
        Some(v) => v,
        None => {
            let (_, all_variables) = SDK::get_all_variables(&key.fingerprint).await?;
            let selected = crate::utils::prompt::prompt_options(
                "Select variables to delete",
                all_variables
                    .iter()
                    .map(|v| format!("{} - ({}) - {}", v.id, v.value.to_string(), v.project_id))
                    .collect::<Vec<String>>(),
            )?;

            if selected.is_empty() {
                return Err(anyhow::anyhow!("No variables selected"));
            }

            selected
                .split(" - ")
                .map(|s| s.to_string())
                .collect::<Vec<String>>()[0]
                .clone()
        }
    };

    SDK::delete_variable(&variable, &key.fingerprint).await?;

    Ok(())
}
