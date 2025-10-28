use pgp::composed::{ArmorOptions, Message, MessageBuilder};
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::types::{CompressionAlgorithm, StringToKey};

pub fn password_encrypt_to_armor(
    input: &str,
    passphrase: &str,
) -> anyhow::Result<String> {
    let mut rng = rand::rngs::OsRng;
    let mut msg = MessageBuilder::from_bytes("", input.as_bytes().to_vec())
        .seipd_v1(&mut rng, SymmetricKeyAlgorithm::AES256);
    msg.compression(CompressionAlgorithm::ZLIB);
    msg.encrypt_with_password(
        StringToKey::new_argon2(&mut rng, 1, 4, 21),
        &passphrase.into(),
    )?;

    let armor = msg.to_armored_string(&mut rng, ArmorOptions::default())?;
    Ok(armor)
}

pub fn password_decrypt_from_armor(
    input: &str,
    passphrase: &str,
) -> anyhow::Result<String> {
    let (msg, _) = Message::from_string(input)?;

    let mut decrypted = msg.decrypt_with_password(&passphrase.into())?;

    if decrypted.is_compressed() {
        decrypted = decrypted.decompress()?;
    }

    Ok(decrypted.as_data_string()?)
}
