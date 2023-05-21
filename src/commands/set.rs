use super::*;
use crate::utils::prompt::prompt_text;
use anyhow::Ok;

/// Clean the cache
#[derive(Parser)]
pub struct Args {
    kvpair: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let (key, value) = match args.kvpair {
        Some(kvpair) => {
            let split = &kvpair.split("=").collect::<Vec<&str>>();
            if split.len() != 2 {
                eprintln!("Error: Invalid key=value pair");
                std::process::exit(1);
            }
            (split[0].to_uppercase().to_string(), split[1].to_string())
        }
        None => {
            let key = match prompt_text("key") {
                Good(key) => key.to_uppercase(),
                Err(_) => {
                    eprintln!("Error: Could not read key");
                    std::process::exit(1);
                }
            };

            let value = match prompt_text("value") {
                Good(value) => value,
                Err(_) => {
                    eprintln!("Error: Could not read value");
                    std::process::exit(1);
                }
            };

            (key, value)
        }
    };

    crate::sdk::Client::set_env(key, value).await?;

    Ok(())
}
