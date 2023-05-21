use crate::{
    controllers::variables::get_variables,
    utils::prompt::{prompt_confirm, prompt_options},
};
use anyhow::Ok;

use reqwest::Client;

use super::*;
use crate::utils::config::*;

/// unset a variable
#[derive(Parser)]
pub struct Args {
    key: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let client = Client::new();
    let config = get_config()?;

    let key = match args.key {
        Some(key) => key.to_uppercase(),
        None => {
            let variables = get_variables().await.unwrap();
            let variables = variables
                .iter()
                .map(|variable| variable.name.clone())
                .collect::<Vec<String>>();
            let key = match prompt_options("key", variables) {
                Good(key) => key.to_uppercase(),
                Err(_) => {
                    eprintln!("Error: Could not read key");
                    std::process::exit(1);
                }
            };

            key
        }
    };

    match prompt_confirm(format!("Are you sure you want to unset {}?", key).as_str()) {
        Good(true) => (),
        _ => {
            println!("Aborting");
            std::process::exit(1);
        }
    }

    let res = client
        .delete(format!(
            "{}/env/{}/{}",
            BASE_URL,
            config.user_id.clone(),
            key
        ))
        .send()
        .await?;

    if res.status().is_success() {
        println!("Successfully unset {}", key);
    } else {
        println!("Error: Could not unset {}", key);
    }

    Ok(())
}
