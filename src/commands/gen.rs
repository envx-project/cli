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
};
use crate::utils::vecu8::ToHex;
use anyhow::Context;
use pgp::types::KeyTrait;
use std::fs;
use std::str;

extern crate keyring;
use keyring::Error as KeyringError;

/// Generate a key using GPG
/// Saves the key to ~/.envcli/keys/<fingerprint>
#[derive(Parser)]
pub struct Args {
    /// Interactive mode
    #[clap(short, long)]
    interactive: bool,

    /// Username
    #[clap(short, long)]
    username: Option<String>,

    /// Your real name
    #[clap(short, long)]
    name: Option<String>,

    /// Your email address
    #[clap(short, long)]
    email: Option<String>,

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
    let mut config = config::get_config().context("Failed to get config")?;
    let settings = config.get_settings()?;

    let name = args
        .name
        .unwrap_or_else(|| prompt_text("What is your name?").unwrap());

    let username = args
        .username
        .unwrap_or_else(|| prompt_text("What is your username?").unwrap());

    let email = args.email.unwrap_or_else(|| prompt_email("email").unwrap());

    match email_validator(&email) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

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

    let key_pair =
        generate_key_pair(name.clone(), email.clone(), passphrase.to_owned())
            .expect("Failed to generate key pair");

    let priv_key = key_pair
        .secret_key
        .to_armored_string(None)
        .expect("Failed to convert private key to armored ASCII string");

    let pub_key = key_pair
        .public_key
        .to_armored_string(None)
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

    let hashed_note =
        generate_hashed_primary_user_id(name.clone(), email.clone());
    let mut key_to_insert: Key = Key {
        fingerprint: fingerprint.clone(),
        note: "".to_string(),
        primary_user_id: format!("{} <{}>", &name, &email),
        hashed_note: hashed_note.clone(),
        pubkey_only: None,
        uuid: None,
    };

    if config.online {
        match SDK::new_user(&username, &pub_key).await {
            Ok(id) => {
                println!("User ID: {}", id);
                key_to_insert.uuid = Some(id);
            }
            Err(_) => {
                eprintln!("Failed to create user on API");
                eprintln!("Continuing with generation...");
                eprintln!("You can create a user later with `envcli upload`");
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
