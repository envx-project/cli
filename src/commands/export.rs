use super::*;
use crate::utils::{config::get_local_or_global_config, key::VecKeyTrait, prompt::prompt_options};

#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    fingerprint: Option<String>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    println!("Exporting keys...");

    let config = get_local_or_global_config().context("Failed to get config")?;

    let fingerprint = match args.fingerprint {
        Some(fingerprint) => fingerprint,
        None => prompt_options("Select key to export", config.keys.all_fingerprints())?.to_string(),
    };

    dbg!(&fingerprint);

    // if let Some(fingerprint) = args.fingerprint {
    //     println!("Exporting key: {}", fingerprint);
    // }

    Ok(())
}
