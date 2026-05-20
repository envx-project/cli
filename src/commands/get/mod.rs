#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

pub mod config;
pub mod project;
pub mod projects;
pub mod variable;

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

commands_enum!(project, config, projects, variable);

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    Commands::exec(args, config).await?;
    Ok(())
}
