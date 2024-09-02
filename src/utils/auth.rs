use crate::utils::config::get_config;
use anyhow::{anyhow, Context};
use chrono::Utc;
use pgp::composed::message::Message;
use pgp::{crypto, ArmorOptions, Deserializable, SignedSecretKey};
use serde::{Deserialize, Serialize};

use super::keyring::try_get_password;

pub async fn get_token(
    fingerprint: &str,
    token: &str,
) -> anyhow::Result<AuthToken> {
    let config = get_config().context("Failed to get config")?;
    let key = config
        .keys
        .iter()
        .find(|k| k.fingerprint.contains(fingerprint))
        .ok_or_else(|| anyhow!("Key not found"))?;

    let key = key.secret_key().context("Failed to get secret key")?;
    let (key, _) = SignedSecretKey::from_string(&key)
        .context("Failed to parse secret key")?;

    let msg = Message::new_literal("none", &Utc::now().to_string());

    let passphrase = try_get_password(fingerprint, &config)?;
    let pw = || passphrase;

    let signature = msg.sign(&key, pw, crypto::hash::HashAlgorithm::SHA3_512);

    let signature = match signature {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to sign API authentication challenge: {}", e);
            if let pgp::errors::Error::Incomplete(_) = e {
                eprintln!("This is most likely due to a missing or incorrect passphrase.");
                println!(
                    "You can view the saved passphrase with 'envx keyring view [fingerprint]'"
                );
                println!("This command is interactive");
                // println!("Or you can check against the saved passphrase with 'envx keyring check -k <fingerprint> -p <passphrase>'");
                // println!("Both of these commands are interactive")
            }

            return Err(anyhow!("Failed to sign API authentication challenge"));
        }
    };

    let signature = match signature.to_armored_string(ArmorOptions::default()) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to convert signature to armored string: {}", e);
            return Err(anyhow!(
                "Failed to convert signature to armored string"
            ));
        }
    };
    let auth_token = AuthToken::new(token.into(), signature);

    Ok(auth_token)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub signature: String,
}

impl AuthToken {
    pub fn new(token: String, signature: String) -> Self {
        Self { token, signature }
    }
}

impl From<AuthToken> for String {
    fn from(auth_token: AuthToken) -> String {
        serde_json::to_string(&auth_token).unwrap()
    }
}

impl std::fmt::Display for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string_token = serde_json::to_string(&self).unwrap();

        write!(f, "{}", string_token)
    }
}
