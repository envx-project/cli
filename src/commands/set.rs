use super::*;
use crate::{
    sdk::SetEnvParams,
    utils::{
        config::get_config,
        prompt::prompt_text,
        rpgp::{encrypt, get_primary_key},
    },
};
use anyhow::{Context, Ok};
use serde_json::json;

/// SET an environment variable with a key=value pair
/// also supports interactive mode
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    fingerprint: Option<String>,

    #[clap(trailing_var_arg = true)]
    kvpairs: Vec<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config()?;
    let pubkey = get_primary_key()
        .context("Failed to get primary key, try generating a new one with `envcli gen`")?;

    if !args.kvpairs.is_empty() {
        for arg in args.kvpairs.clone() {
            let split = &arg.splitn(2, '=').collect::<Vec<&str>>();
            if split.len() != 2 {
                eprintln!("Error: Invalid key=value pair");
                std::process::exit(1);
            }

            let key = split[0].to_uppercase().to_string();
            let value = split[1].to_string();

            set_variable(
                &key,
                &value,
                &pubkey,
                &config.primary_key,
                Some("alphabeta"),
            )
            .await?;
        }

        return Ok(());
    }

    let key = match prompt_text("key") {
        Good(key) => key.to_uppercase(),
        Err(_) => {
            return Err(anyhow::Error::msg("Error: Could not read key"));
        }
    };

    let value = match prompt_text("value") {
        Good(value) => value,
        Err(_) => {
            return Err(anyhow::Error::msg("Error: Could not read value"));
        }
    };

    set_variable(&key, &value, &pubkey, &config.primary_key, None).await?;

    Ok(())
}

async fn set_variable(
    key: &str,
    value: &str,
    pubkey: &str,
    primary_key: &str,
    project_id: Option<&str>,
) -> Result<()> {
    println!("Setting {}={}", key, value);

    let kvpair = json!({
        "key": key,
        "value": value
    });

    let message = encrypt(kvpair.to_string().as_str(), pubkey)?;

    let body = SetEnvParams {
        message,
        allowed_keys: vec![primary_key.to_string()],
        project_id: project_id.map(|id| id.to_owned()),
    };

    crate::sdk::SDK::set_env_old(body).await?;

    Ok(())
}
