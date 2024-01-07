use crate::utils::config::get_config;
use anyhow::{anyhow, Context, Ok};
use chrono::Utc;
use pgp::composed::message::Message;
use pgp::{crypto, Deserializable, SignedSecretKey};

use super::keyring::try_get_password;

pub async fn get_token(fingerprint: &str, token: &str) -> anyhow::Result<String> {
    let config = get_config().context("Failed to get config")?;
    let key = config
        .keys
        .iter()
        .find(|k| k.fingerprint.contains(fingerprint))
        .ok_or_else(|| anyhow!("Key not found"))?;

    let key = key.secret_key().context("Failed to get secret key")?;
    let (key, _) = SignedSecretKey::from_string(&key).context("Failed to parse secret key")?;

    let msg = Message::new_literal("none", &Utc::now().to_string());

    // TODO: get password from user
    let passphrase = try_get_password(fingerprint, &config)?;
    let pw = || passphrase;

    let signature = msg
        .sign(&key, pw, crypto::hash::HashAlgorithm::SHA3_512)
        .context("Failed to sign API authentication challenge")?
        .to_armored_string(None)
        .context("Failed to convert signature to armored string")?;

    let auth_token = serde_json::json!({
        "token": token,
        "signature": signature,
    })
    .to_string();

    Ok(auth_token)
}
