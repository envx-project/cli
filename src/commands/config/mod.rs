#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

use crate::commands_enum;
use clap::Subcommand;

pub mod edit;
pub mod fields;
pub mod get;
pub mod migrate;
pub mod set;
pub mod unset;

/// Configure envx
#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}

commands_enum!(edit, get, set, unset, migrate);

pub async fn command(
    args: Args,
    config: &mut crate::utils::config::Config,
) -> Result<()> {
    Commands::exec(args, config).await?;
    Ok(())
}
