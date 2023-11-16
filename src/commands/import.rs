use super::*;
use crate::utils::prompt::prompt_text;
use crate::utils::rpgp::generate_hashed_primary_user_id;
use crate::utils::vecu8::ToHex;
use crate::utils::{config::get_local_or_global_config, key::Key};
use pgp::{types::KeyTrait, Deserializable};
use std::io::Cursor;

/// Import ascii armored keys from a file
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    path: String,

    #[clap(long)]
    pubkey: bool,

    #[clap(long)]
    secret_key: bool,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    println!("Importing keys...");

    let mut config = get_local_or_global_config().context("Failed to get config")?;

    let file = std::fs::read_to_string(&args.path).context("Failed to read file")?;

    if args.pubkey {
        let buf = Cursor::new(file.clone());
        let (key, _) = pgp::composed::SignedPublicKey::from_armor_single(buf)
            .context("Failed to parse armored key")?;

        let fingerprint = key.fingerprint().to_hex().to_uppercase();

        println!("Importing key: {}", fingerprint);

        let first_user_id = key.details.users.get(0).unwrap().id.id().to_string();

        let (primary_user_id, hashed_note) =
            if only_hex(&first_user_id) && first_user_id.len() == 128 {
                let name = prompt_text("What is the name of the owner of this key?")?;
                let email = prompt_text("What is the email of the owner of this key?")?;

                let hashed_primary_user_id =
                    generate_hashed_primary_user_id(name.clone(), email.clone());

                (format!("{} <{}>", name, email), hashed_primary_user_id)
            } else {
                (first_user_id, "".to_string())
            };

        let key = Key {
            fingerprint,
            note: "".to_string(),
            pubkey_only: Some(true),
            primary_user_id,
            hashed_note,
        };

        dbg!(key.clone());

        return Ok(());
    }

    Ok(())
}

fn only_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_hexdigit())
}
