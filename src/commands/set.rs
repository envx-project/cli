use anyhow::bail;

use super::*;
use crate::{
    sdk::SDK,
    utils::{
        choice::Choice,
        config::get_config,
        kvpair::KVPair,
        // partial_variable::ToParsed,
        prompt::prompt_confirm,
    },
};

/// Set a variable (Interactive)
///
/// Overwrites existing variables if they exist.
#[derive(Parser)]
pub struct Args {
    /// KVPairs
    #[clap(trailing_var_arg = true)]
    kvpairs: Vec<String>,

    /// Key to use for encryption
    #[clap(short, long)]
    key: Option<String>,

    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    if args.kvpairs.is_empty() {
        bail!(
            "{}\n{}",
            "No KV pairs provided".red(),
            "Usage: envx set key=value [key=value]...",
        );
    }

    let config = get_config()?;
    let key = match &args.key {
        Some(k) => k,
        None => &config.primary_key,
    };
    let key = config.get_key(key)?;

    let project_id =
        Choice::try_project(args.project_id, &key.fingerprint).await?;

    if project_id.is_empty() {
        return Err(anyhow::anyhow!("No project ID provided"));
    }

    let (kvpairs, errors): (Vec<KVPair>, Vec<String>) = args
        .kvpairs
        .iter()
        .fold((Vec::new(), Vec::new()), |(mut ok, mut err), k| {
            match k.split_once('=') {
                Some((key, value)) => {
                    ok.push(KVPair::new(key.to_uppercase(), value.into()))
                }
                None => err.push(format!("Invalid KVPair: {}", k)),
            }
            (ok, err)
        });

    errors.iter().for_each(|e| println!("Skipping {}", e));

    if kvpairs.is_empty() {
        return Err(anyhow::anyhow!("No valid KV pairs provided"));
    }

    let variables = SDK::get_variables(&project_id, &key.fingerprint).await?;

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

        let overwrite =
            prompt_confirm("Do you want to override existing variables?")?;

        if !overwrite {
            println!("Aborting...");
            return Ok(());
        }

        println!("Overwriting existing variables...");
        for k in existing_keys {
            let id = k.id.clone();
            SDK::delete_variable(&id, &key.fingerprint).await?;
        }
    }

    let ids = SDK::set_many(kvpairs, &key.fingerprint, &project_id).await?;

    println!("Uploaded {} variables", ids.len());
    println!("IDs: {:?}", ids);

    Ok(())
}
