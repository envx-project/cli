use super::*;
use crate::utils::{
    choice::Choice, config::Config, env_override::apply_env_overrides, loud,
    magic_variables::get_variables_magic,
};
use anyhow::bail;
use std::collections::BTreeMap;

/// Run a local command using variables from the active environment
#[derive(Debug, Parser)]
pub struct Args {
    /// Project ID
    #[arg(short, long)]
    project_id: Option<String>,

    /// Override or add environment variables (KEY=VALUE), repeatable
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    env_override: Vec<String>,

    /// Args to pass to the command
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(args.project_id, &key).await?;

    if project_id.is_empty() {
        return Err(anyhow::anyhow!("No project ID provided"));
    }

    let mut all_variables = BTreeMap::<String, String>::new();
    all_variables.insert("IN_ENVX_SHELL".to_owned(), "true".to_owned());

    let variables = get_variables_magic(&project_id, &key, false).await?;
    let decrypted_count = variables.len();

    for variable in variables {
        all_variables.insert(variable.key, variable.value);
    }

    apply_env_overrides(&mut all_variables, args.env_override)?;

    loud::say(
        config,
        format!("injected {} decrypted vars", decrypted_count),
    );

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
        .envs(all_variables)
        .status()
        .await?;

    if let Some(code) = exit_status.code() {
        // If there is an exit code (process not terminated by signal), exit with that code
        std::process::exit(code);
    }

    Ok(())
}
