use std::io::{BufRead, IsTerminal};

use anyhow::bail;

use super::*;
use crate::{
    sdk::SDK,
    utils::{
        choice::Choice,
        config::Config,
        kvpair::KVPair,
        // partial_variable::ToParsed,
        prompt::prompt_confirm,
    },
};

/// Set a variable (Interactive)
///
/// Overwrites existing variables if they exist.
/// Accepts KEY=VALUE pairs as positional args and/or on stdin
/// (one per line; `#` comments and blank lines ignored).
#[derive(Parser)]
pub struct Args {
    /// KVPairs
    #[arg(trailing_var_arg = true)]
    kvpairs: Vec<String>,

    /// Project ID
    #[arg(short, long)]
    project_id: Option<String>,

    /// Skip confirmation and overwrite existing variables
    #[arg(short, long)]
    yes: bool,

    /// Output uploaded variable IDs as JSON
    #[arg(long)]
    json: bool,
}

/// Read additional KEY=VALUE lines from stdin when it is piped.
/// Blank lines and lines starting with `#` are ignored.
fn read_stdin_kvpairs() -> anyhow::Result<Vec<String>> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let stdin_pairs = read_stdin_kvpairs()?;

    let all_inputs: Vec<String> =
        args.kvpairs.iter().cloned().chain(stdin_pairs).collect();

    if all_inputs.is_empty() {
        bail!(
            "{}\n{}",
            "No KV pairs provided".red(),
            "Usage: envx set key=value [key=value]...  (or pipe KEY=VALUE lines on stdin)",
        );
    }

    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(args.project_id, &key).await?;

    if project_id.is_empty() {
        return Err(anyhow::anyhow!("No project ID provided"));
    }

    let (kvpairs, errors): (Vec<KVPair>, Vec<String>) = all_inputs.iter().fold(
        (Vec::new(), Vec::new()),
        |(mut ok, mut err), k| {
            match k.split_once('=') {
                Some((key, value)) => {
                    ok.push(KVPair::new(key.to_uppercase(), value.into()))
                }
                None => err.push(format!("Invalid KVPair: {}", k)),
            }
            (ok, err)
        },
    );

    errors.iter().for_each(|e| println!("Skipping {}", e));

    if kvpairs.is_empty() {
        return Err(anyhow::anyhow!("No valid KV pairs provided"));
    }

    let variables = SDK::get_variables(&project_id, &key).await?;

    let existing_keys = variables
        .iter()
        .filter(|k| {
            kvpairs
                .iter()
                .any(|kv| kv.key.to_uppercase() == k.value.key.to_uppercase())
        })
        .collect::<Vec<_>>();

    if !existing_keys.is_empty() {
        println!("The following variables already exist:");
        for key in &existing_keys {
            println!(
                "{} - {}={}",
                key.id.green(),
                key.value.key.blue(),
                key.value.value.yellow()
            );
        }

        if !args.yes {
            if !std::io::stdin().is_terminal() {
                bail!(
                    "{}\n{}",
                    "Cannot prompt for overwrite confirmation in a non-interactive terminal.".red(),
                    "Re-run with --yes (-y) to overwrite existing variables.",
                );
            }

            let overwrite =
                prompt_confirm("Do you want to override existing variables?")?;

            if !overwrite {
                println!("Aborting...");
                return Ok(());
            }
        }

        println!("Overwriting existing variables...");
        for k in existing_keys {
            let id = k.id.clone();
            SDK::delete_variable(&id, &key).await?;
        }
    }

    let ids = SDK::set_many(kvpairs, &project_id, &key).await?;

    if args.json {
        println!("{}", serde_json::to_string(&ids)?);
    } else {
        println!("Uploaded {} variables", ids.len());
        println!("IDs: {:?}", ids);
    }

    Ok(())
}
