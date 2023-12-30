use super::*;
use crate::utils::btreemap::ToBTreeMap;
use crate::utils::config::get_config;
use crate::utils::table::Table;
use anyhow::Context;
use anyhow::Result;

/// Get the configuration either as a table or as a JSON output
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    keys_only: bool,

    #[clap(long)]
    json: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;

    if args.json {
        let json = serde_json::to_string_pretty(&config).context("Failed to serialize")?;
        println!("{}", json);
        return Ok(());
    };

    if args.keys_only {
        let key_map = config.keys.to_btreemap()?;
        Table::new("Fingerprint | Key ID".into(), key_map).print()?;
        return Ok(());
    };

    Table::new("Configuration".into(), config.to_btreemap()?).print()?;

    Ok(())
}
