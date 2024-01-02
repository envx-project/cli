use super::*;
use crate::utils::choice::Choice;
use crate::utils::config::get_config;

/// Get all environment variables for a project
#[derive(Parser)]
pub struct Args {
    /// Partial fingerprint of key to use
    #[clap(short, long)]
    key: Option<String>,

    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,

    /// Force a new project to be linked to the current directory, unlinking the current project
    #[clap(short, long)]
    force: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = get_config()?;

    let projects = &config.projects;
    let cwd = std::env::current_dir()?;

    if let Some(project) = projects.iter().find(|p| p.path == cwd) {
        if args.force {
            println!("Forced new project");
            println!("Unlinking current project...");
            let old = config.unset_project()?;
            println!(
                "{} {}",
                "Unset project(s):".green(),
                serde_json::to_string(&old)?
            );
        } else {
            println!("A project is already linked to this directory");
            println!("  Use `envcli unlink` to unlink the current project");
            println!("  Or force a new project with `envcli link --force`");
            println!("{} {}", "Current project:".green(), project.project_id);
            return Ok(());
        }
    }

    let key = config.get_key_or_default(args.key)?;

    let project_id = match args.project_id {
        Some(p) => p,
        None => Choice::choose_project(&key.fingerprint).await?,
    };

    config.set_project(&project_id)?;

    Ok(())
}
