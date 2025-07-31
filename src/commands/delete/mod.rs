#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

use crate::{commands_enum, utils::config::Config};
use clap::Subcommand;

/// Delete a resource. (project, key)
#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

commands_enum!();

pub async fn command(args: Args, config: Config) -> Result<()> {
    Commands::exec(args, config).await?;
    Ok(())
}
