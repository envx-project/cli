#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

pub mod accept;
pub mod create;

use crate::{commands_enum, utils::config::Config};
use clap::Subcommand;

/// Get a resource. (project, key, config)
#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, long)]
    json: bool,
}

commands_enum!(accept, create);

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    Commands::exec(args, config).await?;
    Ok(())
}
