#[allow(unused_imports)]
pub(super) use anyhow::{anyhow, Context, Result};
pub(super) use clap::Parser;
#[allow(unused_imports)]
pub(super) use colored::Colorize;

pub mod add_user;
pub mod add_users;
pub mod delete;
pub mod id;
pub mod info;
pub mod list_users;
pub mod new;
pub mod remove_user;
pub mod rename;

use crate::{commands_enum, utils::config::Config};
use clap::Subcommand;

/// Command group for project related commands
#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, long)]
    json: bool,
}

commands_enum!(
    add_user,
    add_users,
    delete,
    id,
    list_users,
    new,
    remove_user,
    rename,
    info
);

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    Commands::exec(args, config).await?;
    Ok(())
}
