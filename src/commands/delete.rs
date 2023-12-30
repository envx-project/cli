use crate::commands_enum;
use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

// Name collision between the subcommand and the module
use crate::commands::_delete::*;

commands_enum!(project);

pub async fn command(args: Args) -> Result<()> {
    Commands::exec(args).await?;
    Ok(())
}
