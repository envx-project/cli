use crate::utils::config::Config;

use super::*;

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

/// Generate completion script
#[derive(Parser)]
pub struct Args {
    shell: Shell,
}

pub async fn command(args: Args, _config: Config) -> Result<()> {
    generate(
        args.shell,
        &mut self::Args::command(),
        "envx",
        &mut io::stdout(),
    );
    Ok(())
}
