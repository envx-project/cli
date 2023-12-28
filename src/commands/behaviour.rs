use crate::utils::config::get_config;

use super::*;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    exit: bool,

    #[clap(long)]
    tail: bool,

    #[clap(long)]
    detach: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config()?;

    let settings = config.settings.unwrap();

    let behaviour = if args.exit {
        "exit"
    } else if args.tail {
        "tail"
    } else if args.detach {
        "detach"
    } else {
        settings.test.as_str()
    };

    println!("My default behaviour is {}.", behaviour);

    Ok(())
}
