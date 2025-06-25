use super::*;
use crate::utils::prompt::prompt_text;
use crate::{sdk::SDK, utils::config::Config};

/// Rename a project
#[derive(Parser)]
pub struct Args {
    /// Project name
    #[clap(short, long)]
    name: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get()?;
    let key = config.primary_key()?;

    Ok(())
}
