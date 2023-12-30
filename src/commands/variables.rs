use super::*;
use crate::{
    sdk::SDK,
    utils::{btreemap::ToBTreeMap, choice::Choice, config::get_config, table::Table},
};
/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    key: Option<String>,

    #[clap(short, long)]
    project_id: Option<String>,
}

pub async fn command(args: Args, json: bool) -> Result<()> {
    let config = get_config()?;
    let key = config.get_key_or_default(args.key)?;

    let project_id = Choice::try_project(args.project_id, &key.fingerprint).await?;

    let kvpairs = SDK::get_variables_pruned(&project_id, &key.fingerprint).await?;

    let btreemap = kvpairs.to_btreemap()?;

    if !json {
        Table::new("Variables".into(), btreemap).print()?;
    } else {
        println!("{}", serde_json::to_string_pretty(&btreemap)?);
    }

    Ok(())
}
