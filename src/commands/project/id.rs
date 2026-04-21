use super::*;
use crate::utils::config::Config;

#[derive(Parser)]
pub struct Args {
    /// Output the project ID as JSON
    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let project = config.get_project();
    match (project, args.json) {
        (Ok(p), true) => {
            println!("{}", serde_json::json!({ "project_id": p.project_id }))
        }
        (Ok(p), false) => println!("Project ID:\n\t{}", p.project_id),
        (Err(_), true) => {
            println!("{}", serde_json::json!({ "project_id": null }))
        }
        (Err(_), false) => println!("No project set"),
    }
    Ok(())
}
