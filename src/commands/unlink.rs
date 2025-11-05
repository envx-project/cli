use super::*;
use crate::utils::config::Config;

/// Unlink the current project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, config: &mut Config) -> Result<()> {
    let unset = config.unlink_project()?;

    // There should only ever be one project unset
    // but the unset command unsets all projects that match the current directory

    println!("{}", "Unset project(s):".green());
    for project in unset {
        println!("  {}", project);
    }

    Ok(())
}
