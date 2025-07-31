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
#[derive(Parser)]
pub struct Args {
    /// KVPairs
    #[clap(trailing_var_arg = true)]
    kvpairs: Vec<String>,

    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args, config: Config) -> Result<()> {
    if args.kvpairs.is_empty() {
        bail!(
            "{}\n{}",
            "No KV pairs provided".red(),
            "Usage: envx set key=value [key=value]...",
        );
    }

    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(args.project_id, &key).await?;

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

    let variables = SDK::get_variables(&project_id, &key, &config).await?;

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
            SDK::delete_variable(&id, &key).await?;
        }
    }

    let ids = SDK::set_many(kvpairs, &project_id, &key).await?;

    println!("Uploaded {} variables", ids.len());
    println!("IDs: {:?}", ids);

    Ok(())
}
