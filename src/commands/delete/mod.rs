#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

use crate::commands_enum;
use clap::Subcommand;

pub mod key;
pub mod project;

/// Delete a resource. (project, key)
#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

commands_enum!(project, key);

pub async fn command(args: Args) -> Result<()> {
    Commands::exec(args).await?;
    Ok(())
}
