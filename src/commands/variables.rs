use super::*;
use anyhow::Ok;

/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    Ok(())
}
