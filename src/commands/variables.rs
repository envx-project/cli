use std::collections::btree_map;

use crate::{
    sdk::SDK,
    utils::{
        btreemap::ToBTreeMap, config::get_local_or_global_config, kvpair::KVPair, table::Table,
    },
};

use super::*;
use anyhow::Ok;

/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    key: String,

    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_local_or_global_config()?;

    let key = config.get_key(&args.key)?;

    let project_id = match args.project_id {
        Some(p) => p,
        None => todo!("Get project ID from current directory"),
    };

    let (variables, _) =
        SDK::get_variables(&project_id, &key.fingerprint, &key.uuid.clone().unwrap()).await?;

    let btreemap = variables.to_btreemap()?;

    Table::new("Variables".into(), btreemap).print()?;

    Ok(())
}
