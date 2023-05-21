use std::fs;

use crate::utils::prompt::prompt_confirm;

use super::*;

/// Clean the cache
#[derive(Parser)]
pub struct Args {
    /// Deletex without confirmation
    #[clap(short, long)]
    force: bool,
    /// Delete without confirmation
    #[clap(short, long)]
    yes: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let force = args.force || args.yes;

    let output_exists = fs::metadata("./output").is_ok();
    let zipped_exists = fs::metadata("./zipped").is_ok();

    let not_exists = !output_exists && !zipped_exists;

    if not_exists {
        println!("{}", "No files to delete".red());
        return Ok(());
    }

    if !force {
        let input = prompt_confirm("Are you sure you want to delete all files? (y/n)")?;

        if !input {
            eprintln!("{}", "Aborting".red());
            return Ok(());
        }
    }

    if output_exists {
        fs::remove_dir_all("./output")?;
    };

    if zipped_exists {
        fs::remove_dir_all("./zipped")?;
    };

    Ok(())
}
