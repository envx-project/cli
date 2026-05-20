// TODO: add uuid to config after uploading

use super::*;
use crate::constants::MINIMUM_PASSWORD_LENGTH;
use crate::sdk::SDK;
use crate::utils::config::Config;
use crate::utils::key::{validate_passphrase_not_empty, Key};
use crate::utils::keyring::set_password;
use crate::utils::prompt::{is_interactive, prompt_password, prompt_text};
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
    #[arg(short, long)]
    username: Option<String>,

    /// Passphrase to encrypt the key with
    #[arg(short, long)]
    passphrase: Option<String>,

    /// force overwrite of existing key
    #[arg(long = "force", short = 'f')]
    force: bool,

    #[arg(long)]
    export: bool,

    /// Don't upload the key to the API
    #[arg(long = "no-upload")]
    no_upload: bool,

    /// Output the result as JSON (suppresses human-readable output)
    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let settings = config.get_settings();

    if config.primary_key.is_some() {
        if !args.force {
            bail!("A primary key already exists. Use --force to overwrite it.");
        } else {
            println!("Overwriting primary key...");
        }
    }

    if (args.username.is_none() || args.passphrase.is_none())
        && !is_interactive()
    {
        bail!(
            "Cannot prompt for username/passphrase in a non-interactive terminal.\n\
             Pass --username <name> and --passphrase <value>.",
        );
    }

    println!("For your username, do not use your real name, or anything that could be used to identify you.");
    println!("Don't even reuse your username from other services.");
    let username = args
        .username
        .unwrap_or_else(|| prompt_text("Set a username for the key").unwrap());

    let passphrase = args
        .passphrase
        .unwrap_or_else(|| prompt_password("password").unwrap());
    validate_passphrase_not_empty(&passphrase)?;

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

    if !args.json {
        println!("Fingerprint: {}", fingerprint);
    }

    if args.export {
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "fingerprint": fingerprint,
                    "private_key": priv_key,
                    "public_key": pub_key,
                    "exported": true,
                })
            );
        } else {
            println!("PRIVATE:\n{}", priv_key);
            println!("\nPUBLIC:\n{}", pub_key);
        }
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
                if !args.json {
                    println!("User ID: {}", id);
                }
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

    if !args.json {
        println!("Setting primary key to {}...", &fingerprint);
    }

    let key: Key = Key {
        fingerprint: fingerprint.clone(),
        note: "Primary Key".to_string(),
        primary_user_id: user_id(&username),
        pubkey_only: Some(false),
        uuid: uuid.clone(),
    };

    key.verify_passphrase(&passphrase)?;

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

    config.primary_key = Some(key);

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "fingerprint": fingerprint,
                "user_id": uuid,
                "public_key": pub_key,
            })
        );
    }

    Ok(())
}
