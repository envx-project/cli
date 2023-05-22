use super::*;
use crate::utils::encryption::encrypt;

/// Encrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    /// recipient's public key
    recipient: String,

    /// string to encrypt
    text: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    match encrypt(args.recipient, args.text).await {
        Ok(encrypted) => println!("{}", encrypted),
        Err(error) => eprintln!("{}", error),
    }

    Ok(())
}
