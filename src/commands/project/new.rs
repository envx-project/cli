use anyhow::bail;

use super::*;
use crate::utils::prompt::{is_interactive, prompt_text};
use crate::{sdk::SDK, utils::config::Config};

/// Create a new project
#[derive(Parser)]
pub struct Args {
    /// Project name
    #[arg(short, long)]
    name: Option<String>,

    #[arg(long = "no-name")]
    noname: bool,

    #[arg(long = "no-link")]
    nolink: bool,

    #[arg(long, short)]
    force: bool,

    /// Output the new project ID as JSON (suppresses all other output)
    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);

    let name = if args.noname {
        "".to_string()
    } else {
        match args.name {
            Some(n) => n,
            None => {
                if !is_interactive() {
                    bail!(
                        "No --name given and stdin is not a terminal.\n\
                         Pass --name <name> (or --no-name for an unnamed project).",
                    );
                }
                prompt_text("What is the name of this project?")?
            }
        }
    };

    let new_project_id = SDK::new_project(&key, &name).await?;
    if !args.json {
        println!("Created new project with ID: {}", new_project_id);
    }

    // early return if nolink flag is set, we are done already
    if args.nolink {
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "project_id": new_project_id,
                    "linked": false,
                })
            );
        }
        return Ok(());
    }

    if !args.force {
        if let Ok(existing) = config.get_project() {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "project_id": new_project_id,
                        "linked": false,
                        "already_linked_project_id": existing.project_id,
                    })
                );
            } else {
                println!("A project is already linked to this directory");
                println!("  Use `envx unlink` to unlink the current project");
                println!("  Or force link a project with `envx link --force`");
                println!(
                    "{} {}",
                    "Current project:".green(),
                    existing.project_id
                );
            }
            return Ok(());
        }
    }

    if !args.json {
        println!("Linking project...");
    }

    if let Ok(unlinked) = config.unlink_project() {
        if !args.json {
            println!("Unlinked project(s):");
            for project in unlinked {
                println!("  {}", project);
            }
        }
    }
    config.link_project(&new_project_id)?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "project_id": new_project_id,
                "linked": true,
            })
        );
    }

    Ok(())
}
