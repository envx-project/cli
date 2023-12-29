use super::*;
use crate::utils::key::Key;
use crate::utils::prompt::prompt_text;
use crate::utils::rpgp::get_vault_location;
use crate::utils::vecu8::ToHex;
use clap::Subcommand;
use pgp::{types::KeyTrait, Deserializable};
use std::fs;
use std::io::Cursor;

/// Import ascii armored keys from a file
#[derive(Parser, Debug)]
pub struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Import a public key
    Pubkey { path: String },
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let mut vault_path = get_vault_location()?;
    match args.command {
        Commands::Pubkey { path } => {
            let buf = Cursor::new(std::fs::read_to_string(&path).context("Failed to read file")?);
            let (pubkey, _) = pgp::composed::SignedPublicKey::from_armor_single(buf)
                .context("Failed to parse armored key")?;

            let fingerprint = pubkey.fingerprint().to_hex().to_uppercase();

            println!("Importing key: {}", fingerprint);

            let first_user_id = pubkey.details.users.get(0).unwrap().id.id().to_string();

            let (primary_user_id, hashed_note) =
                if only_hex(&first_user_id) && first_user_id.len() == 128 {
                    println!("This key has no user id because it was generated by env-cli.");
                    println!("Please enter the name and email of the owner of this key.");
                    println!("User Id: {}", first_user_id);

                    let name = prompt_text("What is the name of the owner of this key?")?;
                    let email = prompt_text("What is the email of the owner of this key?")?;
                    (format!("{} <{}>", name, email), first_user_id)
                } else {
                    (first_user_id, "".to_string())
                };

            let key = Key {
                fingerprint,
                note: "".to_string(),
                pubkey_only: Some(true),
                primary_user_id,
                hashed_note,
                uuid: None,
            };

            vault_path.push(format!("{}/public.key", &key.fingerprint));
            fs::write(vault_path, pubkey.to_armored_string(None)?)?;

            dbg!(key.clone());
        }
    }

    Ok(())
}

fn only_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}
