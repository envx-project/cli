// TODO: add uuid to config after uploading

use super::*;
use crate::sdk::SDK;
use crate::utils::config::{self};
use crate::utils::key::Key;
// use crate::utils::prompt::prompt_password;
use crate::utils::prompt::{prompt_email, prompt_password, prompt_text};
use crate::utils::rpgp::{
    generate_hashed_primary_user_id, generate_key_pair, get_vault_location, GenerationOptions,
};
use crate::utils::vecu8::ToHex;
use anyhow::Context;
use clap::ValueEnum;
use pgp::types::KeyTrait;
use pgp::KeyType;
use std::fs;

/// Generate a key using GPG
/// Saves the key to ~/.envcli/keys/<fingerprint>
#[derive(Parser)]
pub struct Args {
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

    /// Programatically disable passphrase
    #[clap(long = "nopwd")]
    no_passphrase: bool,

    /// force overwrite of existing key
    #[clap(long = "force", short = 'f')]
    force_overwrite: bool,

    /// Generate another key
    #[clap(long = "new-key")]
    force_generate_new_key: bool,

    #[clap(long)]
    export: bool,

    /// Override whether or not to upload the new key
    #[clap(long)]
    online: Option<bool>,

    /// Use a real user id or a garbled one
    #[clap(long, default_value = "false")]
    real_user_id: bool,

    /// How secure do you want it?
    #[clap(long, default_value = "Rsa2048")]
    algorithm: Algorithm,
}

fn email_validator(email: &str) -> anyhow::Result<(), anyhow::Error> {
    let regex = regex::Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$")
        .context("Failed to create regex for email validation")?;
    if regex.is_match(email) {
        Ok(())
    } else {
        Err(anyhow::Error::msg("Please enter a valid email address"))
    }
}

#[derive(Copy, Clone)]
enum Algorithm {
    Rsa2048,
    Rsa3072,
    Rsa4096,
    Rsa8192,
}

const VALID_ALGORITHMS: &[&str] = &["Rsa2048", "Rsa3072", "Rsa4096", "Rsa8192"];

impl ValueEnum for Algorithm {
    fn from_str(input: &str, ignore_case: bool) -> std::prelude::v1::Result<Self, String> {
        if ignore_case {
            return match input.to_lowercase().as_str() {
                "rsa2048" => Ok(Algorithm::Rsa2048),
                "rsa3072" => Ok(Algorithm::Rsa3072),
                "rsa4096" => Ok(Algorithm::Rsa4096),
                "rsa8192" => Ok(Algorithm::Rsa8192),
                _ => Err(format!(
                    "Invalid algorithm: {}\nValid Algorithms: {}",
                    input,
                    VALID_ALGORITHMS.join(", ")
                )),
            };
        }

        match input {
            "Rsa2048" => Ok(Algorithm::Rsa2048),
            "Rsa3072" => Ok(Algorithm::Rsa3072),
            "Rsa4096" => Ok(Algorithm::Rsa4096),
            "Rsa8192" => Ok(Algorithm::Rsa8192),
            _ => Err(format!(
                "Invalid algorithm: {}\nValid Algorithms: {}",
                input,
                VALID_ALGORITHMS.join(", ")
            )),
        }
    }

    fn value_variants<'a>() -> &'a [Self] {
        &[
            Algorithm::Rsa2048,
            Algorithm::Rsa3072,
            Algorithm::Rsa4096,
            Algorithm::Rsa8192,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Algorithm::Rsa2048 => Some(clap::builder::PossibleValue::new("Rsa2048")),
            Algorithm::Rsa3072 => Some(clap::builder::PossibleValue::new("Rsa3072")),
            Algorithm::Rsa4096 => Some(clap::builder::PossibleValue::new("Rsa4096")),
            Algorithm::Rsa8192 => Some(clap::builder::PossibleValue::new("Rsa8192")),
        }
    }
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = config::get_config().context("Failed to get config")?;

    let online = if let Some(online) = args.online {
        online
    } else {
        config.online
    };

    let name = args
        .name
        .unwrap_or_else(|| prompt_text("What is your name?").unwrap());

    let username = match online {
        true => match args.username {
            Some(username) => Some(username),
            None => {
                let username = prompt_text("Choose a username:").unwrap();
                let username = username.trim();
                if username.is_empty() {
                    None
                } else {
                    Some(username.to_string())
                }
            }
        },
        false => None,
    };

    let username = username.unwrap_or_default();

    let email = args
        .email
        .unwrap_or_else(|| prompt_email("What is your email?").unwrap());

    email_validator(&email).context("Invalid email")?;

    let passphrase = args.passphrase.unwrap_or_else(|| {
        {
            println!("Put nothing for no password");
            prompt_password("Set a password:")
        }
        .unwrap()
    });

    let algorithm = match args.algorithm {
        Algorithm::Rsa2048 => KeyType::Rsa(2048),
        Algorithm::Rsa3072 => KeyType::Rsa(3072),
        Algorithm::Rsa4096 => KeyType::Rsa(4096),
        Algorithm::Rsa8192 => KeyType::Rsa(8192),
    };

    let mut options = GenerationOptions::default();

    options
        .identity(&name, &email)
        .password(&passphrase)
        .algorithm(algorithm);
    if args.real_user_id {
        options.use_real_user_id();
    }
    let built_options = options.build();

    let key_pair = generate_key_pair(built_options).expect("Failed to generate key pair");

    let priv_key = key_pair
        .secret_key
        .to_armored_string(None)
        .expect("Failed to convert private key to armored ASCII string");

    let pub_key = key_pair
        .public_key
        .to_armored_string(None)
        .expect("Failed to convert public key to armored ASCII string");

    let fingerprint: String = key_pair.secret_key.fingerprint().to_hex();

    println!("Fingerprint: {}", fingerprint);

    if args.export {
        println!("PRIVATE:\n{}", priv_key);
        println!("\nPUBLIC:\n{}", pub_key);
        return Ok(());
    }

    let key_dir = get_vault_location()?.join(fingerprint.clone());
    fs::create_dir_all(&key_dir).context("Failed to create key directory")?;

    fs::write(key_dir.join("private.key"), &priv_key).expect("Failed to write private key to file");
    fs::write(key_dir.join("public.key"), &pub_key).expect("Failed to write public key to file");

    let hashed_note = generate_hashed_primary_user_id(&name, &email);
    let key_to_insert: Key = Key {
        fingerprint: fingerprint.clone(),
        note: "".to_string(),
        primary_user_id: format!("{} <{}>", &name, &email),
        hashed_note: hashed_note.clone(),
        pubkey_only: Some(false),
        uuid: None,
    };

    config.keys.push(key_to_insert);

    if config.primary_key.is_empty() {
        println!("Setting primary key to {}...", &fingerprint);
        config.primary_key = fingerprint.clone();
    }

    config.write().context("Failed to write config")?;

    if online {
        let id = SDK::new_user(&username, &pub_key).await?;
        println!("User ID: {}", id);
        config
            .set_uuid(&fingerprint, &id)
            .context("Failed to set UUID")?;
    }

    Ok(())
}
