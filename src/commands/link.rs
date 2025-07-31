use super::*;
use crate::utils::choice::Choice;
use crate::utils::config::Config;

/// Link a project to the current directory
#[derive(Parser)]
pub struct Args {
    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,

    /// Force a new project to be linked to the current directory, unlinking the current project
    #[clap(short, long)]
    force: bool,
}

pub async fn command(args: Args, config: Config) -> Result<()> {
    let projects = &config.projects;
    let cwd = std::env::current_dir()?;

    if let Some(project) = projects.iter().find(|p| p.path == cwd) {
        if args.force {
            println!("Forced new project");
            println!("Unlinking current project...");
            {
                let mut config = config.clone();
                let old = config.unlink_project()?;
                println!(
                    "{} {}",
                    "Unset project(s):".green(),
                    serde_json::to_string(&old)?
                );
            }
        } else {
            println!("A project is already linked to this directory");
            println!("  Use `envx unlink` to unlink the current project");
            println!("  Or force a new project with `envx link --force`");
            println!("{} {}", "Current project:".green(), project.project_id);
            return Ok(());
        }
    }

    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);

    let project_id = match args.project_id {
        Some(p) => p,
        None => Choice::choose_project(&config.projects, &key).await?,
    };

    {
        let mut config = config.clone();
        config.link_project(&project_id)?;
    }

    Ok(())
}
