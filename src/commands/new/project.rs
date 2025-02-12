use super::*;
use crate::utils::prompt::prompt_text;
use crate::{sdk::SDK, utils::config::get_config};

/// Create a new project
#[derive(Parser)]
pub struct Args {
    /// Key
    #[clap(short, long)]
    key: Option<String>,

    /// Project name
    #[clap(short, long)]
    name: Option<String>,

    #[clap(long)]
    nn: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = get_config()?;
    let key = config.get_key_or_default(args.key)?;

    // check if nn flag is set
    if args.nn {
        let new_project_id = SDK::new_project(&key.fingerprint, "").await?;
        println!("Created new project with ID: {}", new_project_id);
        return Ok(());
    }

    let name;
    if args.nn {
        name = "".to_string();
    } else {
        name = args.name.unwrap_or_else(|| {
            prompt_text("What is the name of this project?").unwrap()
        });
    }

    let new_project_id = SDK::new_project(&key.fingerprint, &name).await?;
    println!("Created new project with ID: {}", new_project_id);
    Ok(())
}
