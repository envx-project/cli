use super::*;
use crate::utils::choice::Choice;
use crate::utils::config::Config;
use crate::utils::loud;

/// Link a project to the current directory
#[derive(Parser)]
pub struct Args {
    /// Project ID
    #[arg(short, long)]
    project_id: Option<String>,

    /// Force a new project to be linked to the current directory, unlinking the current project
    #[arg(short, long)]
    force: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let projects = &config.projects;
    let cwd = std::env::current_dir()?;

    if let Some(project) = projects.iter().find(|p| p.path == cwd) {
        if args.force {
            println!("Forced new project");
            println!("Unlinking current project...");
            {
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

    config.link_project(&project_id)?;

    loud::say(
        config,
        format!(
            "linked {} → project {}",
            tilde_path(&cwd),
            short_id(&project_id),
        ),
    );

    Ok(())
}

fn tilde_path(p: &std::path::Path) -> String {
    if let Some(home) = home::home_dir() {
        if let Ok(rel) = p.strip_prefix(&home) {
            return format!("~/{}", rel.display());
        }
    }
    p.display().to_string()
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}
