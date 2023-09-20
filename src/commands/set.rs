use super::*;
use crate::utils::prompt::prompt_text;
use anyhow::Ok;

/// SET an environment variable with a key=value pair
/// also supports interactive mode
#[derive(Parser)]
pub struct Args {
    #[clap(trailing_var_arg = true)]
    kvpairs: Vec<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    // if args.kvpairs.len() >= 1 {
    //     for arg in args.kvpairs.clone() {
    //         let split = &arg.splitn(2, "=").collect::<Vec<&str>>();
    //         if split.len() != 2 {
    //             eprintln!("Error: Invalid key=value pair");
    //             std::process::exit(1);
    //         }

    //         let key = split[0].to_uppercase().to_string();
    //         let value = split[1].to_string();

    //         println!("Setting {}={}", key, value);
    //         crate::sdk::Client::set_env(key, value).await?;
    //     }

    //     return Ok(());
    // }

    // let key = match prompt_text("key") {
    //     Good(key) => key.to_uppercase(),
    //     Err(_) => {
    //         return Err(anyhow::Error::msg("Error: Could not read key"));
    //     }
    // };

    // let value = match prompt_text("value") {
    //     Good(value) => value,
    //     Err(_) => {
    //         return Err(anyhow::Error::msg("Error: Could not read value"));
    //     }
    // };

    // crate::sdk::Client::set_env(key, value).await?;

    Ok(())
}
