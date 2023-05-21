use super::*;
use crate::utils::config::*;
use crate::utils::prompt::prompt_text;
use anyhow::Ok;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Clean the cache
#[derive(Parser)]
pub struct Args {
    kvpair: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct SetParams {
    password: String,
    value: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let client = Client::new();
    let config = get_config()?;

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

    let params = SetParams {
        password: config.password.clone(),
        value: value.clone(),
    };

    let res = client
        .post(format!("{}/env/{}/{}", BASE_URL, config.user_id, key))
        .json::<SetParams>(&params)
        .send()
        .await?;

    if res.status().is_success() {
        println!("Successfully set {}={}", key, value);
    } else {
        eprintln!("Error: Could not set {}={}", key, value);
        std::process::exit(1);
    }

    Ok(())
}
