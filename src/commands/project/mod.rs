#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

pub mod add_user;
pub mod remove_user;

use crate::commands_enum;
use clap::Subcommand;

/// Command group for project related commands
#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(global = true, long)]
    json: bool,
}

commands_enum!(add_user, remove_user);

pub async fn command(args: Args) -> Result<()> {
    Commands::exec(args).await?;
    Ok(())
}
