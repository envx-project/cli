use super::*;
use crate::utils::config;
use crate::utils::e::{generate_key_pair, get_vault_location};
use crate::utils::prompt::{prompt_email, prompt_password, prompt_text};
use anyhow::Context;
use pgp::types::KeyTrait;
use std::fs;
use std::str;

/// Generate a key using GPG
/// Saves the key to ~/.envcli/keys/<fingerprint>
#[derive(Parser)]
pub struct Args {
    /// Interactive mode
    #[clap(short, long)]
    interactive: bool,

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

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let mut config = config::get_config().context("Failed to get config")?;

    let name = args
        .name
        .unwrap_or_else(|| prompt_text("What is your name?").unwrap());
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

    let key_pair = generate_key_pair(name, email, passphrase).expect("Failed to generate key pair");

    let priv_key = key_pair
        .secret_key
        .to_armored_string(None)
        .expect("Failed to convert private key to armored ASCII string");
    let pub_key = key_pair
        .public_key
        .to_armored_string(None)
        .expect("Failed to convert public key to armored ASCII string");

    let fingerprint: String = key_pair
        .secret_key
        .fingerprint()
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect();

    println!("Fingerprint: {}", fingerprint);

    let key_dir = get_vault_location()?.join(fingerprint.clone());
    fs::create_dir_all(&key_dir).context("Failed to create key directory")?;

    fs::write(key_dir.join("private.key"), priv_key).expect("Failed to write private key to file");
    fs::write(key_dir.join("public.key"), pub_key).expect("Failed to write public key to file");

    config.keys.push(fingerprint.clone());

    if config.primary_key.is_empty() {
        config.primary_key = fingerprint;
    }

    config::write_config(&config).context("Failed to write config")?;

    Ok(())
}
