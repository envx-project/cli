use keyring::{Entry as Keyring, Result as KeyringResult};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use super::{config::Config, prompt::prompt_password};

const SERVICE: &'static str = "envx";

fn get_session_path(fingerprint: &str) -> PathBuf {
    std::env::temp_dir().join(format!("envx-{}", fingerprint))
}

pub fn set_password(fingerprint: &str, password: &str) -> KeyringResult<()> {
    let keyring = Keyring::new(SERVICE, fingerprint)?;

    let expiration = SystemTime::now() + Duration::from_secs(60 * 60 * 24 * 30);
    let exp_bytes = bincode::serialize(&expiration).unwrap();
    fs::File::create(get_session_path(&fingerprint))
        .unwrap()
        .write_all(&exp_bytes)
        .unwrap();

    keyring.set_password(password)
}

pub fn get_password(fingerprint: &str) -> anyhow::Result<String> {
    let expiry = fs::read(get_session_path(&fingerprint));
    if expiry.is_err() {
        clear_password(&fingerprint)?;
        return Err(anyhow::anyhow!("No session found"));
    }

    let expiry = expiry.unwrap();
    let expiry: SystemTime = bincode::deserialize(&expiry).unwrap();

    if expiry < SystemTime::now() {
        clear_password(&fingerprint)?;
        return Err(anyhow::anyhow!("Session expired"));
    }

    let keyring = Keyring::new(SERVICE, fingerprint)?;
    let password = keyring.get_password()?;
    return Ok(password);
}

pub fn clear_password(fingerprint: &str) -> KeyringResult<()> {
    let keyring = Keyring::new(SERVICE, fingerprint)?;
    keyring.delete_password()
}

pub fn try_get_password(fingerprint: &str, config: &Config) -> anyhow::Result<String> {
    let password = get_password(fingerprint);

    match password {
        Ok(p) => Ok(p),
        Err(e) => {
            eprintln!("Failed to get password: {}", e);
            let key = config.get_key(&fingerprint)?;
            println!("Enter password for key {}", key);
            let password = prompt_password("Password: ")?;

            match set_password(&fingerprint, &password) {
                Err(e) => {
                    eprintln!("Failed to set password: {}", e);
                }
                _ => {}
            }
            Ok(password)
        }
    }
}
