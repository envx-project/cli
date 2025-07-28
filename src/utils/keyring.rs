use super::config::Config;
use crate::utils::settings::KeyringExpiry;
use anyhow::bail;
use keyring::{Entry as Keyring, Result as KeyringResult};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime},
};

const SERVICE: &str = "envx";

fn get_session_path(fingerprint: &str) -> PathBuf {
    std::env::temp_dir().join(format!("envx-{}", fingerprint))
}

pub fn set_password(
    fingerprint: &str,
    password: &str,
    expiry: KeyringExpiry,
) -> KeyringResult<()> {
    let keyring = Keyring::new(SERVICE, fingerprint)?;

    if expiry == KeyringExpiry::Never {
        return keyring.set_password(password);
    }

    let days: u64 = match expiry {
        KeyringExpiry::Days(d) => d.into(),
        _ => unreachable!(),
    };

    let expiration =
        SystemTime::now() + Duration::from_secs(days * 24 * 60 * 60);
    let exp_bytes = bincode::serialize(&expiration).unwrap();
    fs::File::create(get_session_path(fingerprint))
        .unwrap()
        .write_all(&exp_bytes)
        .unwrap();

    keyring.set_password(password)
}

pub fn get_password(config: &Config) -> anyhow::Result<String> {
    let settings = config.get_settings();

    let fingerprint = &config.primary_key()?.fingerprint;

    if let Some(password) = &config.primary_key_password {
        return Ok(password.clone());
    }

    match settings.get_keyring_expiry() {
        KeyringExpiry::Days(_) => {
            let expiry = fs::read(get_session_path(fingerprint));
            let expiry = match expiry {
                Ok(e) => e,
                Err(_) => {
                    clear_password(fingerprint)?;
                    bail!("No session found");
                }
            };

            let expiry: SystemTime = bincode::deserialize(&expiry)?;

            if expiry < SystemTime::now() {
                clear_password(fingerprint)?;
                bail!("Session expired");
            }
        }
        _ => {
            println!("No keyring expiry set");
        }
    }

    let keyring = Keyring::new(SERVICE, fingerprint)?;
    let password = keyring.get_password()?;
    Ok(password)
}

pub fn clear_password(fingerprint: &str) -> KeyringResult<()> {
    let keyring = Keyring::new(SERVICE, fingerprint)?;
    keyring.delete_credential()
}
