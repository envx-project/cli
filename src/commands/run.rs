use super::*;
use crate::utils::choice::Choice;
use anyhow::bail;
use std::collections::BTreeMap;

/// Run a local command using variables from the active environment
#[derive(Debug, Parser)]
pub struct Args {
    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,

    /// Key to use for signing
    #[clap(short, long)]
    key: Option<String>,
    /// Args to pass to the command
    #[clap(trailing_var_arg = true)]
    args: Vec<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let config = crate::utils::config::get_config()?;
    let key = match args.key {
        Some(k) => k.to_owned(),
        None => config.primary_key.clone(),
    };
    let key = config.get_key(&key)?;

    let project_id = Choice::try_project(args.project_id, &key.fingerprint).await?;

    if project_id.is_empty() {
        return Err(anyhow::anyhow!("No project ID provided"));
    }

    let mut all_variables = BTreeMap::<String, String>::new();
    all_variables.insert("IN_ENVCLI_SHELL".to_owned(), "true".to_owned());

    let variables = crate::sdk::SDK::get_variables_pruned(&project_id, &key.fingerprint).await?;

    for variable in variables {
        all_variables.insert(variable.key, variable.value);
    }

    // a bit janky :/
    ctrlc::set_handler(move || {
        // do nothing, we just want to ignore CTRL+C
        // this is for `rails c` and similar REPLs
    })?;

    let mut args = args.args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    if args.is_empty() {
        bail!("No command provided");
    }

    let child_process_name = match std::env::consts::OS {
        "windows" => {
            args.insert(0, "/C");
            "cmd"
        }
        _ => args.remove(0),
    };

    let exit_status = tokio::process::Command::new(child_process_name)
        .args(args)
        .envs(all_variables.iter().map(|v| (v.0.clone(), v.1.clone())))
        .status()
        .await?;

    if let Some(code) = exit_status.code() {
        // If there is an exit code (process not terminated by signal), exit with that code
        std::process::exit(code);
    }

    Ok(())
}
