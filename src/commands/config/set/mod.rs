#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

use crate::{commands_enum, utils::config::Config};
use clap::Subcommand;

pub mod keyring_expiry;
pub mod unsafe_password;
pub mod password_command;

/// Delete a resource. (project, key)
#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

commands_enum!(unsafe_password, keyring_expiry, password_command);

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    Commands::exec(args, config).await?;
    Ok(())
}
