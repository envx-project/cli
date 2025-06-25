use super::*;
use crate::utils::prompt::prompt_text;
use crate::{sdk::SDK, utils::config::Config};

/// Create a new project
#[derive(Parser)]
pub struct Args {
    /// Project name
    #[clap(short, long)]
    name: Option<String>,

    #[clap(long = "no-name")]
    noname: bool,

    #[clap(long = "no-link")]
    nolink: bool,

    #[clap(long, short)]
    force: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get()?;
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);

    let name = if args.noname {
        "".to_string()
    } else {
        args.name.unwrap_or_else(|| {
            prompt_text("What is the name of this project?").unwrap()
        })
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

    let mut config = config;
    match config.unlink_project() {
        Ok(unlinked) => {
            println!("Unlinked project(s):");
            for project in unlinked {
                println!("  {}", project);
            }
        }
        Err(_) => {}
    }
    config.link_project(&new_project_id)?;
    config.write()?;

    Ok(())
}
