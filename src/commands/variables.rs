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
    Ok(())
}
