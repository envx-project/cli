//! Versioned, signed payloads. Decryption is never treated as sender authentication.
use super::{key::UnlockedKey, rpgp};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use pgp::{
    composed::{
        ArmorOptions, Deserializable, Message, MessageBuilder, SignedPublicKey,
        SignedSecretKey,
    },
    crypto::hash::HashAlgorithm,
    types::KeyDetails,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Payload {
    Text(String),
    Variables(BTreeMap<String, String>),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub version: u8,
    pub server: String,
    pub id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub sender_fingerprint: String,
    pub recipient_fingerprint: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub payload: Payload,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub version: u8,
    pub id: String,
    pub creator_id: String,
    pub creator_fingerprint: String,
    pub redeemer_id: String,
    pub redeemer_fingerprint: String,
}

pub fn public_key(armor: &str) -> Result<SignedPublicKey> {
    let (key, _) =
        SignedPublicKey::from_string(armor).context("Invalid public key")?;
    key.verify().context("Invalid public key self-signature")?;
    Ok(key)
}
pub fn fingerprint(armor: &str) -> Result<String> {
    Ok(hex::encode(public_key(armor)?.fingerprint().as_bytes()))
}
pub fn sign<T: Serialize>(value: &T, key: &UnlockedKey) -> Result<String> {
    let secret = SignedSecretKey::try_from(key)?;
    sign_with_secret(value, &secret, &key.password)
}
fn sign_with_secret<T: Serialize>(
    value: &T,
    secret: &SignedSecretKey,
    password: &str,
) -> Result<String> {
    let mut builder =
        MessageBuilder::from_bytes("", serde_json::to_vec(value)?);
    builder.sign(
        &secret.primary_key,
        password.into(),
        HashAlgorithm::Sha3_512,
    );
    Ok(
        builder
            .to_armored_string(rand::rngs::OsRng, ArmorOptions::default())?,
    )
}
pub fn verify<T: serde::de::DeserializeOwned>(
    armor: &str,
    public: &str,
) -> Result<T> {
    let key = public_key(public)?;
    let (mut message, _) =
        Message::from_string(armor).context("Invalid signed message")?;
    let data = message
        .as_data_string()
        .context("Invalid signed message contents")?;
    message
        .verify(&key)
        .context("Message signature verification failed")?;
    serde_json::from_str(&data).context("Invalid signed payload")
}
pub fn encrypt(
    envelope: &Envelope,
    key: &UnlockedKey,
    recipient: &str,
) -> Result<String> {
    let signed = sign(envelope, key)?;
    let recipient = public_key(recipient)?;
    let sender = SignedPublicKey::try_from(key)?;
    rpgp::encrypt(&signed, &[recipient, sender])
}
pub fn decrypt(
    ciphertext: &str,
    key: &UnlockedKey,
    sender: &str,
) -> Result<Envelope> {
    let signed = rpgp::decrypt(
        ciphertext,
        &SignedSecretKey::try_from(key)?,
        &key.password,
    )?;
    let envelope: Envelope = verify(&signed, sender)?;
    if envelope.version != 1 {
        bail!("Unsupported message version");
    }
    if envelope.expires_at.is_some_and(|time| time <= Utc::now()) {
        bail!("Message expired");
    }
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_payload_rejects_wrong_signer_and_unsigned_content() {
        let generate = || {
            let params = pgp::composed::SecretKeyParamsBuilder::default()
                .key_type(pgp::composed::KeyType::Rsa(2048))
                .can_sign(true)
                .can_encrypt(true)
                .primary_user_id("test".into())
                .build()
                .unwrap();
            params
                .generate(rand::rngs::OsRng)
                .unwrap()
                .sign(rand::rngs::OsRng, &"".into())
                .unwrap()
        };
        let alice = generate();
        let bob = generate();
        let alice_public = SignedPublicKey::from(alice.clone())
            .to_armored_string(ArmorOptions::default())
            .unwrap();
        let bob_public = SignedPublicKey::from(bob)
            .to_armored_string(ArmorOptions::default())
            .unwrap();
        let receipt = Receipt {
            version: 1,
            id: uuid::Uuid::new_v4().to_string(),
            creator_id: "alice".into(),
            creator_fingerprint: "a".into(),
            redeemer_id: "bob".into(),
            redeemer_fingerprint: "b".into(),
        };
        let signed = sign_with_secret(&receipt, &alice, "").unwrap();
        assert_eq!(verify::<Receipt>(&signed, &alice_public).unwrap(), receipt);
        assert!(verify::<Receipt>(&signed, &bob_public).is_err());
        let unsigned = MessageBuilder::from_bytes(
            "",
            serde_json::to_vec(&receipt).unwrap(),
        )
        .to_armored_string(rand::rngs::OsRng, ArmorOptions::default())
        .unwrap();
        assert!(verify::<Receipt>(&unsigned, &alice_public).is_err());
    }
}
