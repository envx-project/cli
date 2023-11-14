use super::*;
use crate::utils::{config::get_local_or_global_config, rpgp::encrypt};
use anyhow::Context;
use serde_json::json;

/// set a local variable
#[derive(Debug, Parser)]
pub struct Args {
    #[clap(short, long)]
    fingerprint: Option<String>,

    #[clap(trailing_var_arg = true)]
    kvpairs: Vec<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_local_or_global_config()?;
    let pubkey = config
        .primary_key()
        .context("Failed to get primary key, try generating a new one with `envcli gen`")?;

    let mut file = std::fs::File::open(".envcli.vault")
        .context("Failed to open .envcli.vault, try running `envcli init`")?;

    let mut variables = match serde_json::from_reader::<_, Vec<String>>(&mut file) {
        Ok(parsed) => parsed,
        Err(_) => vec![],
    };

    if args.kvpairs.len() >= 1 {
        for arg in args.kvpairs.clone() {
            let split = &arg.splitn(2, "=").collect::<Vec<&str>>();
            if split.len() != 2 {
                eprintln!("Error: Invalid key=value pair");
                std::process::exit(1);
            }

            let key = split[0].to_uppercase().to_string();
            let value = split[1].to_string();

            let kvpair = json!({
                "key": key,
                "value": value
            });

            let message = encrypt(kvpair.to_string().as_str(), &pubkey)?;

            println!("{}", &message);

            variables.push(message);
        }

        dbg!(&variables);

        std::fs::write(".envcli.vault", serde_json::to_string_pretty(&variables)?)
            .context("Failed to write to .envcli.vault")?;

        return Ok(());
    }

    Ok(())
}
