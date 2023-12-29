use crate::utils::config::get_local_or_global_config;
use anyhow::{anyhow, Context, Ok};
use chrono::Utc;
use pgp::composed::message::Message;
use pgp::{crypto, Deserializable, SignedSecretKey};

pub async fn get_token(key: &str, token: &str) -> anyhow::Result<String> {
    let config = get_local_or_global_config().context("Failed to get config")?;
    let key = config
        .keys
        .iter()
        .find(|k| k.fingerprint.contains(&key))
        .ok_or_else(|| anyhow!("Key not found"))?;

    let key = SignedSecretKey::from_string(&key.secret_key()?)?.0;

    let msg = Message::new_literal("none", &Utc::now().to_string());
    let pw = || "asdf".to_string();
    let signature = msg.sign(&key, pw, crypto::hash::HashAlgorithm::SHA3_512)?;

    let auth_token = serde_json::json!({
        "token": token,
        "signature": signature.to_armored_string(None)?
    })
    .to_string();

    Ok(auth_token)
}
