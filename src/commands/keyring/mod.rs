#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

use crate::{commands_enum, utils::config::Config};
use clap::Subcommand;

pub mod clear;
pub mod view;

/// Interact with the envx keyring. All commands are interactive.
#[derive(Parser)]
pub struct Args {
    #[args(subcommand)]
    command: Commands,
}

commands_enum!(view, clear);

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    Commands::exec(args, config).await?;
    Ok(())
}
