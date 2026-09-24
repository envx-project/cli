use super::*;
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

    /// Include stored passwords and password commands in output
    #[arg(long)]
    reveal: bool,
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

    let mut display = serde_json::to_value(&*config)?;
    if !args.reveal {
        for field in ["primary_key_password", "primary_key_command"] {
            if !display[field].is_null() {
                display[field] = serde_json::json!("<redacted>");
            }
        }
    }
    if args.json {
        let json = serde_json::to_string_pretty(&display)
            .context("Failed to serialize")?;
        println!("{}", json);
        return Ok(());
    };

    Table::new(
        "Configuration".into(),
        display
            .as_object()
            .context("Invalid config shape")?
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect(),
    )
    .print()?;

    Ok(())
}
