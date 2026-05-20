use super::*;
use crate::utils::btreemap::ToBTreeMap;
use crate::utils::config::Config;
use crate::utils::table::Table;
use anyhow::Context;
use anyhow::Result;
use std::collections::BTreeMap;

/// Get the configuration either as a table or as a JSON output
#[derive(Parser)]
pub struct Args {
    /// Show only the primary key
    #[arg(short, long)]
    key: bool,

    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    if args.key {
        let key = config.primary_key.as_ref();

        if args.json {
            let json = serde_json::to_string_pretty(&key)
                .context("Failed to serialize")?;
            println!("{}", json);
            return Ok(());
        }

        let mut map = BTreeMap::new();
        if let Some(key) = key {
            map.insert(
                key.fingerprint.chars().skip(30).collect(),
                key.primary_user_id.clone(),
            );
        }
        Table::new("Fingerprint | Key ID".into(), map).print()?;
        return Ok(());
    }

    if args.json {
        let json = serde_json::to_string_pretty(&config)
            .context("Failed to serialize")?;
        println!("{}", json);
        return Ok(());
    };

    Table::new("Configuration".into(), config.to_btreemap()?).print()?;

    Ok(())
}
