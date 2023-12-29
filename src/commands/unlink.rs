use super::*;
use crate::utils::config::get_config;

/// Unset the current project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    let mut config = get_config()?;
    config.unset_project()?;
    println!("Unset project");
    Ok(())
}
