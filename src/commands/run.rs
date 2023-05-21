use super::*;
use crate::sdk::Client;
use anyhow::bail;
/// Run a local command using variables from the active environment
#[derive(Debug, Parser)]
pub struct Args {
    /// Args to pass to the command
    #[clap(trailing_var_arg = true)]
    args: Vec<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let variables = Client::get_variables().await?;

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
        .envs(variables.iter().map(|v| (v.name.clone(), v.value.clone())))
        .status()
        .await?;

    if let Some(code) = exit_status.code() {
        // If there is an exit code (process not terminated by signal), exit with that code
        std::process::exit(code);
    }

    Ok(())
}
