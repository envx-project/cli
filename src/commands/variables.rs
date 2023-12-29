use crate::{
    sdk::SDK,
    utils::{btreemap::ToBTreeMap, config::get_config, table::Table},
};

use super::*;
use anyhow::Ok;

/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    key: Option<String>,

    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config()?;

    let key = config.get_key_or_default(args.key)?;

    let project_id = match args.project_id {
        Some(p) => p,
        None => todo!("Get project ID from current directory"),
    };

    let kvpairs =
        SDK::get_variables_pruned(&project_id, &key.fingerprint, &key.uuid.clone().unwrap())
            .await?;

    let btreemap = kvpairs.to_btreemap()?;

    Table::new("Variables".into(), btreemap).print()?;

    Ok(())
}
