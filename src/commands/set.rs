use std::io::{BufRead, IsTerminal};

use anyhow::bail;

use super::*;
use crate::{
    sdk::SDK,
    utils::{
        cache,
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
    /// Show full IDs and diagnostic details
    #[arg(long)]
    pub verbose: bool,

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
        out.push(line.trim_start().to_owned());
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

    // Reject the whole batch before writing; malformed lines may themselves be secrets.
    let kvpairs = parse_inputs(&all_inputs)?;

    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(config, args.project_id, &key).await?;

    if project_id.is_empty() {
        return Err(anyhow::anyhow!("No project ID provided"));
    }

    let variables = SDK::get_variables(config, &project_id, &key).await?;

    let existing_keys = variables
        .iter()
        .filter(|k| {
            kvpairs
                .iter()
                .any(|kv| kv.key.to_uppercase() == k.value.key.to_uppercase())
        })
        .collect::<Vec<_>>();

    if !existing_keys.is_empty() {
        eprintln!("The following variables already exist:");
        for key in &existing_keys {
            eprintln!(
                "{} - {}",
                key.id.green(),
                crate::utils::messaging::safe(&key.value.key).blue()
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
                eprintln!("Aborting...");
                if args.json {
                    println!("[]");
                }
                return Ok(());
            }
        }

        eprintln!("Overwriting existing variables...");
    }

    let replace_ids = existing_keys.iter().map(|v| v.id.clone()).collect();
    let ids =
        SDK::replace_many(config, kvpairs, &project_id, &key, replace_ids)
            .await?;

    // Re-fetch and update cache
    if let Ok(fresh_vars) = SDK::get_variables(config, &project_id, &key).await
    {
        if let Ok(projects) = SDK::list_projects(config, &key).await {
            if let Some(proj) =
                projects.iter().find(|p| p.project_id == project_id)
            {
                if let Err(e) = cache::write_cache(
                    config,
                    &project_id,
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

    if args.json {
        println!("{}", serde_json::to_string(&ids)?);
    } else {
        println!("Uploaded {} variables", ids.len());
        if args.verbose {
            println!("Variable IDs: {}", ids.join(", "));
        }
    }

    Ok(())
}

fn parse_inputs(inputs: &[String]) -> Result<Vec<KVPair>> {
    let mut names = std::collections::HashSet::new();
    inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let (name, value) = input.split_once('=').with_context(|| {
                format!("Input {} must be KEY=VALUE", index + 1)
            })?;
            if name.is_empty() || name.chars().any(char::is_control) {
                bail!("Input {} has an invalid variable name", index + 1);
            }
            let name = name.to_uppercase();
            if !names.insert(name.clone()) {
                bail!("Input {} repeats a variable name", index + 1);
            }
            Ok(KVPair::new(name, value.into()))
        })
        .collect()
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn set_input_fails_closed_without_echoing_secrets() {
        let error = parse_inputs(&[
            "GOOD=public".into(),
            "private-token-without-equals".into(),
        ])
        .unwrap_err()
        .to_string();
        assert!(error.contains("Input 2"));
        assert!(!error.contains("private-token"));
        assert!(parse_inputs(&["A=one".into(), "a=two".into()]).is_err());
        let pair = parse_inputs(&["TOKEN= keeps spaces ".into()]).unwrap();
        assert_eq!(pair[0].value, " keeps spaces ");
    }
}
