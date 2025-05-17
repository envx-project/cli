use super::*;
use crate::utils::config::Config;

/// Unset the current project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    let mut config = Config::get()?;

    let unset = config.unlink_project()?;
    config.write()?;

    // There should only ever be one project unset
    // but the unset command unsets all projects that match the current directory

    println!("{}", "Unset project(s):".green());
    for project in unset {
        println!("  {}", project);
    }

    Ok(())
}
