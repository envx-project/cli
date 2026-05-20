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
        variable::DecryptedVariable,
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

    enforce_project_caps(config, &variables, &existing_keys, &kvpairs)?;

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

fn kvpair_bytes(key: &str, value: &str) -> u64 {
    (key.len() + value.len()) as u64
}

fn enforce_project_caps(
    config: &Config,
    existing: &[DecryptedVariable],
    overwrites: &[&DecryptedVariable],
    incoming: &[KVPair],
) -> Result<()> {
    let settings = config.get_settings();
    let max_count = settings.get_max_variables_per_project() as usize;
    let max_bytes = settings.get_max_project_bytes();

    let final_count = existing
        .len()
        .saturating_sub(overwrites.len())
        .saturating_add(incoming.len());
    if final_count > max_count {
        bail!(
            "Variable count cap exceeded: this `envx set` would put the project at {} variable(s) (cap: {}).\n\
             Raise it with `envx config set settings.max_variables_per_project <n>`.",
            final_count,
            max_count,
        );
    }

    let existing_bytes: u64 = existing
        .iter()
        .map(|v| kvpair_bytes(&v.value.key, &v.value.value))
        .sum();
    let overwrite_bytes: u64 = overwrites
        .iter()
        .map(|v| kvpair_bytes(&v.value.key, &v.value.value))
        .sum();
    let new_bytes: u64 = incoming
        .iter()
        .map(|kv| kvpair_bytes(&kv.key, &kv.value))
        .sum();
    let final_bytes = existing_bytes
        .saturating_sub(overwrite_bytes)
        .saturating_add(new_bytes);
    if final_bytes > max_bytes {
        bail!(
            "Project size cap exceeded: this `envx set` would put the project at {} bytes of plaintext (cap: {}).\n\
             Raise it with `envx config set settings.max_project_bytes <bytes>`.",
            final_bytes,
            max_bytes,
        );
    }

    Ok(())
}
