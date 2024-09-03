use super::*;
use crate::utils::choice::Choice;
use crate::utils::prompt;
use crate::{sdk::SDK, utils::config::get_config};

/// Unset (delete) an environment variable
#[derive(Parser)]
pub struct Args {
    /// Variable to unset
    #[clap(short, long)]
    variable: Option<String>,

    /// Key to use
    #[clap(short, long)]
    key: Option<String>,

    #[clap(short, long)]
    project_id: Option<String>,

    #[clap(short, long, default_value_t = false)]
    all: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;
    let key = config.get_key_or_default(args.key)?;
    let project_id = match args.all {
        true => None,
        false => {
            Some(Choice::try_project(args.project_id, &key.fingerprint).await?)
        }
    };

    let variable = match args.variable {
        Some(v) => v,
        None => {
            let variables = if let Some(project_id) = project_id {
                SDK::get_variables(&project_id, &key.fingerprint).await?
            } else {
                SDK::get_all_variables(&key.fingerprint).await?
            };

            prompt::prompt_options("Select variables to delete", variables)?.id
        }
    };

    SDK::delete_variable(&variable, &key.fingerprint).await?;

    Ok(())
}
