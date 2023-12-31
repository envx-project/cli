#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

use crate::commands_enum;
use clap::Subcommand;

// pub mod algo;

/// Set a configuration setting
#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(global = true, long)]
    json: bool,
}

#[allow(unreachable_code)]
commands_enum!();

pub async fn command(_args: Args) -> Result<()> {
    // Commands::exec(args).await?;

    println!("This command is not yet implemented.");

    Ok(())
}
