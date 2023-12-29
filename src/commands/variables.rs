use crate::sdk::SDK;

use super::*;
use anyhow::Ok;

/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    key: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let project_id = "be38c68e-5003-4493-a7b8-5653e8db26a6";

    let variables = SDK::get_variables(
        project_id,
        &args.key,
        "dd2bc69b-cabe-4e35-903e-ef76485bd757",
    )
    .await?;

    dbg!(variables);

    Ok(())
}
