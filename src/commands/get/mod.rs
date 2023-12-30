#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

pub mod config;
pub mod keys;
pub mod project;
// pub mod projects;

use crate::commands_enum;
use clap::Subcommand;

/// Get a resource. (project, key, config)
#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(global = true, long)]
    json: bool,
}

commands_enum!(project, config, keys);

pub async fn command(args: Args) -> Result<()> {
    Commands::exec(args).await?;
    Ok(())
}
