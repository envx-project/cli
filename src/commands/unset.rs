use super::*;
use crate::utils::prompt::{prompt_confirm, prompt_options};

/// UNSET an environment variable by key
/// also supports interactive mode
#[derive(Parser)]
pub struct Args {
    key: Option<String>,

    #[clap(short, long)]
    force: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let key = match args.key {
        Some(key) => key.to_uppercase(),
        None => {
            let variables = crate::sdk::Client::get_variables().await?;
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

    if !args.force {
        match prompt_confirm(format!("Are you sure you want to unset {}?", key.red()).as_str()) {
            Good(true) => (),
            _ => {
                println!("Aborting");
                std::process::exit(1);
            }
        }
    }

    crate::sdk::Client::delete_env(key).await?;

    Ok(())
}
