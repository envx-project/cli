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
    println!("Created new project with ID: {}", new_project_id);

    // early return if nolink flag is set, we are done already
    if args.nolink {
        return Ok(());
    }

    if !args.force {
        match config.get_project() {
            Ok(_) => {
                println!("A project is already linked to this directory");
                println!("  Use `envx unlink` to unlink the current project");
                println!("  Or force link a project with `envx link --force`");
                println!(
                    "{} {}",
                    "Current project:".green(),
                    config.get_project()?.project_id
                );
                return Ok(());
            }
            Err(_) => {}
        }
    }

    println!("Linking project...");

    match config.unlink_project() {
        Ok(unlinked) => {
            println!("Unlinked project(s):");
            for project in unlinked {
                println!("  {}", project);
            }
        }
        Err(_) => {} // do nothing, we don't need to unlink
    }
    config.link_project(&new_project_id)?;

    Ok(())
}
