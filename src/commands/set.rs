use anyhow::bail;

use super::*;
use crate::{
    sdk::SDK,
    utils::{choice::Choice, config::get_config, kvpair::KVPair},
};

/// Set a variable
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

    let ids = SDK::set_many(kvpairs, &key.fingerprint, &project_id).await?;

    println!("Uploaded {} variables", ids.len());
    println!("IDs: {:?}", ids);

    Ok(())
}
