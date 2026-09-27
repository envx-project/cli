use super::*;
use crate::utils::config::Config;
use anyhow::Context;
use anyhow::Result;

/// Get the configuration as KEY=VALUE lines or as JSON
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

        if let Some(key) = key {
            println!("fingerprint={}", key.fingerprint);
            println!("primary_user_id={}", key.primary_user_id);
        }
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

    for (k, v) in display.as_object().context("Invalid config shape")? {
        match v {
            serde_json::Value::String(v) => println!("{k}={v}"),
            v => println!("{k}={v}"),
        }
    }

    Ok(())
}
