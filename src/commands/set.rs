use super::*;
use crate::{
    sdk::SDK,
    utils::{config::get_local_or_global_config, kvpair::KVPair},
};
use anyhow::Ok;

/// Set a variable
#[derive(Parser)]
pub struct Args {
    /// Key to use for signing
    #[clap(short, long)]
    key: Option<String>,

    /// KVPairs
    #[clap(trailing_var_arg = true)]
    kvpairs: Vec<String>,

    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_local_or_global_config()?;
    let key = match args.key {
        Some(k) => k.to_owned(),
        None => config.primary_key.clone(),
    };
    let key = config.get_key(&key)?;

    let project_id = match args.project_id {
        Some(p) => p,
        None => config.default_project_id.unwrap_or_default(),
    };

    if project_id.is_empty() {
        return Err(anyhow::anyhow!("No project ID provided"));
    }

    let kvpairs = args
        .kvpairs
        .iter()
        .map(|k| {
            let (key, value) = k.split_once('=').unwrap();
            KVPair::new(key.to_uppercase(), value.to_string())
        })
        .collect::<Vec<KVPair>>();

    let ids = SDK::set_many(
        kvpairs,
        &key.fingerprint,
        &project_id,
        &key.uuid.clone().unwrap(),
    )
    .await?;

    println!("Uploaded {} variables", ids.len());
    println!("IDs: {:?}", ids);

    Ok(())
}
