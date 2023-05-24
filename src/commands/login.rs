use super::*;
use crate::utils::config::*;
use crate::utils::prompt::prompt_text;
use anyhow::Ok;
use base64::Engine;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::result::Result::Ok as Good;

/// login to the service
#[derive(Parser)]
pub struct Args {
    /// Optional password, cli will prompt if not provided
    password: Option<String>,

    /// Generate a password automatically
    #[clap(short, long)]
    generate: bool,

    /// Force login, overwriting existing credentials
    #[clap(short, long)]
    force: bool,
}

#[derive(Serialize, Deserialize)]
struct User {
    id: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    if !args.force {
        match get_config() {
            Good(config) => {
                if (config.user_id != "") && (config.password != "") {
                    eprintln!("Error: You are already logged in");
                    std::process::exit(1);
                }
            }
            Err(_) => (),
        }
    }

    let user_id = crate::sdk::Client::create_user().await?.id;

    let password = match args.password {
        Some(password) => password,
        None => match args.generate {
            true => {
                let mut array: [u8; 64] = [0; 64];
                OsRng.try_fill_bytes(&mut array[..])?;
                let password = base64::engine::general_purpose::STANDARD.encode(&array);
                password
            }

            false => match prompt_text("password") {
                Good(password) => password,
                Err(_) => {
                    eprintln!("Error: Could not read password");
                    std::process::exit(1);
                }
            },
        },
    };

    let newconfig = Config {
        user_id: user_id.clone(),
        password: password.clone(),
        primary_key: "".to_string(),
        keys: vec![],
    };

    write_config(&newconfig)?;

    println!();
    println!("Logged in as {}", user_id.yellow());
    if args.generate {
        println!("Password: {}", password.red());
    }

    Ok(())
}
