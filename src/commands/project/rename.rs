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
    todo!()
    // let config = Config::get()?;
    // let key = config.get_key_or_default(args.key)?;
    //
    // Ok(())
}
