use super::*;
use crate::utils::config::Config;

/// Export a public or secret key
#[derive(Parser)]
pub struct Args {
    /// Export the secret key
    #[arg(short, long = "secret-key")]
    secret_key: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;

    let key = if args.secret_key {
        key.secret_key_str()?
    } else {
        key.public_key_str()?
    };

    println!("{}", key);

    Ok(())
}
