// TODO: add uuid to config after uploading

use super::*;
use crate::constants::MINIMUM_PASSWORD_LENGTH;
use crate::sdk::SDK;
use crate::utils::config::Config;
use crate::utils::key::Key;
use crate::utils::keyring::set_password;
use crate::utils::prompt::{prompt_password, prompt_text};
use crate::utils::rpgp::{generate_key_pair, get_vault_location, user_id};
use crate::utils::vecu8::ToHex;
use anyhow::{bail, Context};
use pgp::composed::ArmorOptions;
use pgp::types::KeyDetails;
use std::fs;

extern crate keyring;
use keyring::Error as KeyringError;

/// Generate a key using GPG
/// Saves the key to ~/.config/envx/keys/<fingerprint>
#[derive(Parser)]
pub struct Args {
    /// Username for the key. Do NOT use your real name, or anything that could be used to identify you.
    #[clap(short, long)]
    username: Option<String>,

    /// Passphrase to encrypt the key with
    #[clap(short, long)]
    passphrase: Option<String>,

    /// force overwrite of existing key
    #[clap(long = "force", short = 'f')]
    force: bool,

    #[clap(long)]
    export: bool,

    /// Don't upload the key to the API
    #[clap(long = "no-upload")]
    no_upload: bool,
}

pub async fn command(args: Args, config: Config) -> Result<()> {
    let settings = config.get_settings();

    if config.primary_key.is_some() {
        if !args.force {
            bail!("A primary key already exists. Use --force to overwrite it.");
        } else {
            println!("Overwriting primary key...");
        }
    }

    println!("For your username, do not use your real name, or anything that could be used to identify you.");
    println!("Don't even reuse your username from other services.");
    let username = args
        .username
        .unwrap_or_else(|| prompt_text("Set a username for the key").unwrap());

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

    let key_pair = generate_key_pair(&username, passphrase.to_owned())
        .expect("Failed to generate key pair");

    let priv_key = key_pair
        .secret_key
        .to_armored_string(ArmorOptions::default())
        .expect("Failed to convert private key to armored ASCII string");

    let pub_key = key_pair
        .public_key
        .to_armored_string(ArmorOptions::default())
        .expect("Failed to convert public key to armored ASCII string");

    let fingerprint = key_pair.secret_key.fingerprint().as_bytes().to_hex();

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
                    "Please submit a bug report at https://github.com/envx-project/cli/issues/new"
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

    let uuid = if !args.no_upload {
        match SDK::new_user(&username, &pub_key).await {
            Ok(id) => {
                println!("User ID: {}", id);
                Some(id)
            }
            Err(_) => {
                eprintln!("Failed to create user on API");
                eprintln!("Continuing with generation...");
                eprintln!("You can create a user later with `envx upload`");
                None
            }
        }
    } else {
        None
    };

    println!("Setting primary key to {}...", &fingerprint);

    let key: Key = Key {
        fingerprint,
        note: "Primary Key".to_string(),
        primary_user_id: user_id(&username),
        pubkey_only: Some(false),
        uuid,
    };

    let mut config = config;
    config.primary_key = Some(key);

    Ok(())
}
