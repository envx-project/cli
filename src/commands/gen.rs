// TODO: add uuid to config after uploading

use super::*;
use crate::sdk::SDK;
use crate::utils::config::{self};
use crate::utils::key::Key;
use crate::utils::keyring::set_password;
// use crate::utils::prompt::prompt_password;
use crate::constants::MINIMUM_PASSWORD_LENGTH;
use crate::utils::prompt::{prompt_email, prompt_password, prompt_text};
use crate::utils::rpgp::{
    generate_hashed_primary_user_id, generate_key_pair, get_vault_location,
    user_id,
};
use crate::utils::vecu8::ToHex;
use anyhow::Context;
use pgp::types::KeyTrait;
use pgp::ArmorOptions;
use std::fs;
use std::str;

extern crate keyring;
use keyring::Error as KeyringError;

/// Generate a key using GPG
/// Saves the key to ~/.config/envx/keys/<fingerprint>
#[derive(Parser)]
pub struct Args {
    /// Interactive mode
    #[clap(short, long)]
    interactive: bool,

    /// Nickname for the key. Do NOT use your real name.
    #[clap(short, long)]
    nickname: Option<String>,

    /// Passphrase to encrypt the key with
    #[clap(short, long)]
    passphrase: Option<String>,

    /// force overwrite of existing key
    #[clap(long = "force", short = 'f')]
    force_overwrite: bool,

    /// Generate another key
    #[clap(long = "new-key")]
    force_generate_new_key: bool,

    #[clap(long)]
    export: bool,
}

fn email_validator(email: &str) -> anyhow::Result<(), anyhow::Error> {
    let regex =
        regex::Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$")
            .context("Failed to create regex for email validation")?;

    match regex.is_match(email) {
        true => Ok(()),
        false => Err(anyhow::Error::msg("Please enter a valid email address")),
    }

    // if regex.is_match(email) {
    //     Ok(())
    // } else {
    //     Err(anyhow::Error::msg("Please enter a valid email address"))
    // }
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = config::Config::get().context("Failed to get config")?;
    let settings = config.get_settings()?;

    let nickname = args
        .nickname
        .unwrap_or_else(|| prompt_text("Set a nickname for the key").unwrap());

    let passphrase = args
        .passphrase
        .unwrap_or_else(|| prompt_password("password").unwrap());

    if settings.warn_on_short_passwords
        && passphrase.len() < MINIMUM_PASSWORD_LENGTH
    {
        eprintln!("WARNING: Your password is short");
        eprintln!("This is not recommended");
        eprintln!("You can disable this warning with `envx config --no-warn-on-short-passwords`");
    }

    let key_pair = generate_key_pair(&nickname, passphrase.to_owned())
        .expect("Failed to generate key pair");

    let priv_key = key_pair
        .secret_key
        .to_armored_string(ArmorOptions::default())
        .expect("Failed to convert private key to armored ASCII string");

    let pub_key = key_pair
        .public_key
        .to_armored_string(ArmorOptions::default())
        .expect("Failed to convert public key to armored ASCII string");

    let fingerprint = key_pair.secret_key.fingerprint().to_hex();

    let result =
        set_password(&fingerprint, &passphrase, settings.get_keyring_expiry());

    if let Err(e) = result {
        match e {
            KeyringError::TooLong(_, length) => {
                eprintln!("Password is too long to store in keyring");
                eprintln!("Length: {}", length);
                eprintln!("Continuing with generation...");
            }
            KeyringError::Invalid(_, _) => {
                eprintln!("Password is invalid");
                eprintln!("Continuing with generation...");
            }
            KeyringError::Ambiguous(c) => {
                eprintln!(
                    "Somehow there are multiple keys with the same fingerprint"
                );
                eprintln!("Keys: {:?}", c);
                eprintln!(
                    "Please submit a bug report at https://github.com/env-cli/rusty-cli/issues/new"
                );
                eprintln!("Continuing with generation...");
            }
            _ => {
                eprintln!("Failed to set password in keyring");
                eprintln!("{}", e);
                eprintln!("Continuing with generation...");
            }
        }
    }

    println!("Fingerprint: {}", fingerprint);

    if args.export {
        println!("PRIVATE:\n{}", priv_key);
        println!("\nPUBLIC:\n{}", pub_key);
        return Ok(());
    }

    let key_dir = get_vault_location()?.join(fingerprint.clone());

    fs::create_dir_all(&key_dir).context("Failed to create key directory")?;

    fs::write(key_dir.join("private.key"), &priv_key)
        .expect("Failed to write private key to file");
    fs::write(key_dir.join("public.key"), &pub_key)
        .expect("Failed to write public key to file");

    let mut key_to_insert: Key = Key {
        fingerprint: fingerprint.clone(),
        note: "".to_string(),
        primary_user_id: user_id(&nickname),
        pubkey_only: None,
        uuid: None,
    };

    if config.online {
        match SDK::new_user(&nickname, &pub_key).await {
            Ok(id) => {
                println!("User ID: {}", id);
                key_to_insert.uuid = Some(id);
            }
            Err(_) => {
                eprintln!("Failed to create user on API");
                eprintln!("Continuing with generation...");
                eprintln!("You can create a user later with `envx upload`");
            }
        };
    }

    config.keys.push(key_to_insert);

    if config.primary_key.is_empty() {
        println!("Setting primary key to {}...", &fingerprint);
        config.primary_key = fingerprint;
    }

    config.write().context("Failed to write config")?;

    Ok(())
}
