use anyhow::bail;

use super::*;
use crate::utils::cache;
use crate::utils::choice::Choice;
use crate::utils::prompt::{
    is_interactive, prompt_confirm_with_default, prompt_multi_options,
};
use crate::{sdk::SDK, utils::config::Config};

/// Unset (delete) environment variables
#[derive(Parser)]
pub struct Args {
    /// Variable names to unset (positional, repeatable)
    #[arg(trailing_var_arg = true)]
    variables: Vec<String>,

    /// Variable to unset (single; kept for backwards compatibility)
    #[arg(short, long)]
    variable: Option<String>,

    /// Key to use
    #[arg(short, long)]
    key: Option<String>,

    #[arg(short, long)]
    project_id: Option<String>,

    /// Unset variables across all projects
    #[arg(short, long, default_value_t = false)]
    all: bool,

    /// Skip confirmation prompt
    #[arg(short, long)]
    yes: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);
    let project_id = match args.all {
        true => None,
        false => Some(Choice::try_project(args.project_id, &key).await?),
    };

    let mut named: Vec<String> = args.variables;
    if let Some(v) = args.variable {
        named.push(v);
    }

    let variable_ids: Vec<String> = if !named.is_empty() {
        let all_vars = match &project_id {
            Some(pid) => SDK::get_variables(pid, &key).await?,
            None => SDK::get_all_variables(&key).await?,
        };
        let wanted: std::collections::HashSet<String> =
            named.iter().map(|s| s.to_uppercase()).collect();

        let matched: Vec<String> = all_vars
            .into_iter()
            .filter(|v| wanted.contains(&v.value.key.to_uppercase()))
            .map(|v| v.id)
            .collect();

        if matched.is_empty() {
            bail!("No matching variables found for: {}", named.join(", "));
        }

        if !args.yes {
            if !is_interactive() {
                bail!(
                    "{}\n{}",
                    "Refusing to delete variables without confirmation in a non-interactive terminal.".red(),
                    "Re-run with --yes (-y) to confirm deletion.",
                );
            }
            let confirmed = prompt_confirm_with_default(
                &format!("Delete {} variable(s)?", matched.len()),
                false,
            )?;
            if !confirmed {
                println!("Aborting...");
                return Ok(());
            }
        }

        matched
    } else {
        if !is_interactive() {
            bail!(
                "{}\n{}",
                "No variables given and stdin is not a terminal.".red(),
                "Pass variable names as positional args (e.g. `envx unset FOO BAR`) or use --all with --yes.",
            );
        }

        let mut variables = match &project_id {
            Some(pid) => SDK::get_variables(pid, &key).await?,
            None => SDK::get_all_variables(&key).await?,
        };

        variables.sort_by(|a, b| a.value.key.cmp(&b.value.key));
        prompt_multi_options("Select variables to delete:", variables)?
            .into_iter()
            .map(|v| v.id)
            .collect()
    };

    for variable in &variable_ids {
        SDK::delete_variable(variable, &key).await?;
    }

    // Re-fetch and update cache for the affected project
    if let Some(pid) = &project_id {
        if let Ok(fresh_vars) = SDK::get_variables(pid, &key).await {
            if let Ok(projects) = SDK::list_projects(&key).await {
                if let Some(proj) =
                    projects.iter().find(|p| &p.project_id == pid)
                {
                    if let Err(e) = cache::write_cache(
                        pid,
                        &proj.project_name,
                        &fresh_vars,
                        &key,
                    ) {
                        eprintln!(
                            "warning: failed to update variable cache: {}",
                            e
                        );
                    }
                }
            }
        }
    }

    println!("Deleted {} variable(s)", variable_ids.len());

    Ok(())
}
