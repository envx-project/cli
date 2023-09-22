use super::*;
use crate::utils::{
    config::get_config,
    rpgp::{decrypt, get_vault_location},
};
use anyhow::{Context, Result};
use hex::ToHex;
use pgp::{composed, Deserializable, SignedSecretKey};
use std::{io::Cursor, vec};

/// Decrypt a string using GPG
#[derive(Parser)]
pub struct Args {}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let config = get_config().context("Failed to get config")?;

    let msg = r#"-----BEGIN PGP MESSAGE-----

wcBMA9uvY2/sNELdAQf+O7K/Z7CmZgGdEkO61a3N2WJHeDsU3L1F0eqt8JbMUVdH
7JsXAu5MlObRhGHnNKuVTt5jXnDH6guoFDsK58FI8rGaHKGzAI1Ri2Q78Cqdrbf1
s5aNcsU/aNg/Sw69fMW/UDNAVG9RVEkO6Fu8yNaLdKM3SSAYYhJWv59f47ufjBQN
r/NJT0h7Rpi3X/zfdxsEanaeg7C+ej+deEQXp6HomMGvIV495WxIDGePAGYJ6dj8
TtZI4r8SinUE0bxSnbGPSAfHJuPd94gSrtzY36k5d97y9I+xrV778I+LtiDaA6BE
tmzlMobvCrT4qNaOMVIH1yilV8JcSisS37dGu/QuhtI9Aejfuc6wnem3ytJP7Ilp
nL6CPN1+K0GBqAJcEJbaDYvTeoRkuxHpu2inmnD17iiSjhIfDOsK2ypW6j7ypA==
=EHv0
-----END PGP MESSAGE-----"#;

    let buf = Cursor::new(msg);

    let (msg, _) = composed::message::Message::from_armor_single(buf)
        .context("Failed to convert &str to armored message")?;

    let recipients = msg
        .get_recipients()
        .iter()
        .map(|e| e.encode_hex_upper())
        .collect::<Vec<String>>();

    let keys = config.keys.clone();

    let mut available_keys: Vec<String> = vec![];

    for key in keys.iter().map(|k| k.fingerprint.clone()) {
        let it_fits = recipients.iter().any(|r| key.contains(r));
        if it_fits {
            available_keys.push(key);
        }
    }

    if available_keys.len() == 0 {
        eprintln!("No keys available to decrypt this message");
        return Ok(());
    }

    let primary_key = config.primary_key.clone();

    let (key, fingerprint) = if available_keys.iter().any(|k| k.contains(&primary_key)) {
        println!("Using primary key");
        get_key(primary_key)
    } else {
        println!("Using first available key: {}", available_keys[0].clone());
        get_key(available_keys[0].clone())
    };

    println!("Using key: {}", fingerprint);

    println!("Decrypting...\n\n");

    let decrypted = decrypt(
        &msg.to_armored_string(None).unwrap(),
        &key,
        String::from("asdf"),
    )?;

    println!("{}", decrypted);

    Ok(())
}

fn get_key(fingerprint: String) -> (SignedSecretKey, String) {
    let key_dir = get_vault_location().unwrap().join(fingerprint.clone());
    let priv_key = std::fs::read_to_string(key_dir.join("private.key")).unwrap();
    let (seckey, _) = SignedSecretKey::from_string(priv_key.as_str()).unwrap();

    (seckey, fingerprint)
}
