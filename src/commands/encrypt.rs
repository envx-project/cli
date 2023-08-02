use super::*;

/// Encrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    // /// recipient's public key
    // recipient: String,

    // /// string to encrypt
    // text: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let fingerprint = "734956E603EC58A94843A27D42449196796A2EEB";
    let passphrase = "joever";
    let keypair = crate::utils::rpgp::read_vault(fingerprint, passphrase)?;

    // encrypt some arbitrary data
    let data = b"hello world";

    let encrypted = crate::utils::rpgp::encrypt(
        keypair,
        "971DBD09E27B4D4C06393604EFB7449DD97A113C",
        "cock and ball torture",
    )?;
    Ok(())
}

// pub async fn command(args: Args, _json: bool) -> Result<()> {
//     match encrypt(args.recipient, args.text).await {
//         Ok(encrypted) => println!("{}", encrypted),
//         Err(error) => eprintln!("{}", error),
//     }

//     Ok(())
// }
