use super::*;
use crate::utils::config::{self, Key};
use crate::utils::e::{
    generate_key_pair, generate_primary_user_id, get_vault_location, hash_string,
};
use crate::utils::prompt::{prompt_email, prompt_password, prompt_text};
use anyhow::Context;
use pgp::types::KeyTrait;
use reqwest::Client;
use serde_json::json;
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

    let key_pair = generate_key_pair(name.clone(), email.clone(), passphrase)
        .expect("Failed to generate key pair");

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

    fs::write(key_dir.join("private.key"), &priv_key).expect("Failed to write private key to file");
    fs::write(key_dir.join("public.key"), &pub_key).expect("Failed to write public key to file");

    let hashed_note = generate_primary_user_id(name.clone(), email.clone());
    let key_to_insert: Key = Key {
        fingerprint: fingerprint.clone(),
        note: "".to_string(),
        primary_user_id: format!("{} <{}>", &name, &email),
        hashed_note: hashed_note.clone(),
    };

    config.keys.push(key_to_insert);

    if config.primary_key.is_empty() {
        println!("Setting primary key to {}...", &fingerprint);
        config.primary_key = fingerprint.clone();
    }

    config::write_config(&config).context("Failed to write config")?;

    let client = Client::new();

    let url = "http://localhost:3000";

    let pubkey_hash = hash_string(&pub_key);

    let body = json!({
        "public_key": &pub_key,
        "fingerprint": &fingerprint,
        "primary_user_id": hashed_note.clone(),
        "public_key_hash": pubkey_hash,
    });

    let response = client.post(url).json(&body).send().await?;

    let status = response.status();
    println!("Response status: {}", status);

    Ok(())
}
