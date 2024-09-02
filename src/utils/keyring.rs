use super::{
    config::{get_config, Config},
    prompt::prompt_password,
};
use crate::utils::settings::KeyringExpiry;
use crate::{
    constants::MINIMUM_PASSWORD_LENGTH, utils::prompt::prompt_confirm,
};
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

pub fn get_password(fingerprint: &str) -> anyhow::Result<String> {
    let config = get_config()?;
    let settings = config.get_settings()?;

    if fingerprint == config.primary_key {
        if let Some(password) = config.primary_key_password {
            return Ok(password);
        }
    }

    if settings.get_keyring_expiry_days().is_some() {
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

    let keyring = Keyring::new(SERVICE, fingerprint)?;
    let password = keyring.get_password()?;
    Ok(password)
}

pub fn clear_password(fingerprint: &str) -> KeyringResult<()> {
    let keyring = Keyring::new(SERVICE, fingerprint)?;
    keyring.delete_credential()
}

pub fn try_get_password(
    fingerprint: &str,
    config: &Config,
) -> anyhow::Result<String> {
    let password = get_password(fingerprint);

    match password {
        Ok(p) => Ok(p),
        Err(e) => {
            let settings = config.get_settings()?;

            eprintln!("Failed to get password: {}", e);
            let key = config.get_key(fingerprint)?;
            println!("Enter password for key {}", key);
            let password = prompt_password("Password: ")?;
            if settings.warn_on_short_passwords
                && password.len() < MINIMUM_PASSWORD_LENGTH
            {
                eprintln!(
                    "This password is shorter than 8 characters. Are you sure you want to proceed?"
                );
                let confirm = prompt_confirm("Continue?")?;
                if !confirm {
                    bail!("Aborted")
                }
            }

            let expiry = settings.get_keyring_expiry();

            if let Err(e) = set_password(fingerprint, &password, expiry) {
                eprintln!("Failed to set password: {}", e);
            }

            Ok(password)
        }
    }
}
