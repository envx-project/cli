use crate::commands_enum;
use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,

    #[clap(global = true, long)]
    json: bool,
}

use crate::commands::_new::*;

commands_enum!(project);

pub async fn command(args: Args) -> Result<()> {
    Commands::exec(args).await?;
    Ok(())
}
