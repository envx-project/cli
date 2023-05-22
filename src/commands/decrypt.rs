use super::*;
use crate::utils::encryption::decrypt;

/// Decrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    /// string to encrypt
    text: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    match decrypt(args.text).await {
        Ok(decrypted) => println!("{}", decrypted),
        Err(error) => eprintln!("{}", error),
    }

    Ok(())
}
