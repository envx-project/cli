use crate::utils::{
    config::{get_config, write_config, Config},
    prompt::prompt_select,
};

use super::*;

/// Set options in the settings.json. It will break if you edit it manually and do it wrong
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    online: Option<bool>,

    #[clap(short, long)]
    url: Option<String>,

    #[clap(long = "pk-value")]
    primary_key_value: Option<String>,

    #[clap(short = 'p', long)]
    set_primary_key: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config()?;

    let primary_key = match args.primary_key_value {
        Some(s) => s,
        None => {
            if args.set_primary_key {
                prompt_select(
                    "Select the primary key you want to use",
                    config
                        .keys
                        .clone()
                        .iter()
                        .map(|e| e.fingerprint.as_str())
                        .collect::<Vec<&str>>(),
                )?
                .into()
            } else {
                config.primary_key.clone()
            }
        }
    };

    let new_config = Config {
        salt: config.salt,
        keys: config.keys,
        online: args.online.unwrap_or(config.online),
        primary_key,
        sdk_url: match args.url {
            Some(url) => Some(url),
            None => match config.sdk_url {
                Some(url) => Some(url),
                None => None,
            },
        },
    };

    write_config(&new_config)?;

    Ok(())
}
