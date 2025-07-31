use super::*;
use crate::utils::config::Config;

#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, config: Config) -> Result<()> {
    let project = config.get_project();
    match project {
        Ok(p) => println!("Project ID:\n\t{}", p.project_id),
        Err(_) => println!("No project set"),
    }
    Ok(())
}
