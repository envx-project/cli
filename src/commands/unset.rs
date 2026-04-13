use super::*;
use crate::utils::choice::Choice;
use crate::utils::prompt;
use crate::{sdk::SDK, utils::config::Config};

/// Unset (delete) an environment variable
#[derive(Parser)]
pub struct Args {
    /// Variable to unset
    #[arg(short, long)]
    variable: Option<String>,

    /// Key to use
    #[arg(short, long)]
    key: Option<String>,

    #[arg(short, long)]
    project_id: Option<String>,

    #[arg(short, long, default_value_t = false)]
    all: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);
    let project_id = match args.all {
        true => None,
        false => Some(Choice::try_project(args.project_id, &key).await?),
    };

    let variables = match args.variable {
        Some(v) => vec![v],
        None => {
            let mut variables = if let Some(project_id) = project_id {
                SDK::get_variables(&project_id, &key).await?
            } else {
                SDK::get_all_variables(&key).await?
            };

            variables.sort_by(|a, b| a.value.key.cmp(&b.value.key));
            prompt::prompt_multi_options(
                "Select variables to delete:",
                variables,
            )?
            .into_iter()
            .map(|v| v.id)
            .collect()
        }
    };

    for variable in &variables {
        SDK::delete_variable(variable, &key).await?;
    }

    Ok(())
}
