use super::config::Config;
use super::state::StateStore;
use crate::utils::settings::KeyringExpiry;
use anyhow::{bail, Context};
use keyring::{Entry as Keyring, Result as KeyringResult};
use std::time::{Duration, SystemTime};

const SERVICE: &str = "envx";

fn state() -> KeyringResult<StateStore> {
    Config::load()
        .and_then(|config| StateStore::open(&config))
        .map_err(|error| keyring::Error::PlatformFailure(error.into()))
}

pub fn set_password(
    fingerprint: &str,
    password: &str,
    expiry: KeyringExpiry,
) -> KeyringResult<()> {
    let keyring = Keyring::new(SERVICE, fingerprint)?;

    let store = state()?;
    let expiration = match expiry {
        KeyringExpiry::Never => None,
        KeyringExpiry::Days(days) => Some(
            SystemTime::now()
                + Duration::from_secs(u64::from(days) * 24 * 60 * 60),
        ),
    };
    // Save the expiry first so a failed state write cannot leave a newly
    // stored credential without its expiry.
    store
        .set_session_expiry(fingerprint, expiration)
        .map_err(|error| keyring::Error::PlatformFailure(error.into()))?;
    keyring.set_password(password)
}

pub fn get_password(config: &Config) -> anyhow::Result<String> {
    let settings = config.get_settings();

    let fingerprint = &config.primary_key()?.fingerprint;

    if let Some(mut command) = config.primary_key_command.clone() {
        if command.is_empty() {
            bail!("No command provided");
        }

        let first = command.remove(0);
        let output = std::process::Command::new(first)
            .args(command)
            .output()
            .context("Failed to run password command")?;
        if !output.status.success() {
            bail!("Command failed");
        }
        let password = String::from_utf8(output.stdout)?;
        return Ok(password.trim_end_matches('\n').to_string());
    }

    if let Some(password) = &config.primary_key_password {
        return Ok(password.clone());
    }

    if let KeyringExpiry::Days(_) = settings.get_keyring_expiry() {
        let expiry = StateStore::open(config)?.session_expiry(fingerprint)?;
        if expiry.map_or(true, |expiry| expiry < SystemTime::now()) {
            // Legacy /tmp files are untrusted and intentionally not imported.
            match clear_password(fingerprint) {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(error) => return Err(error.into()),
            }
            bail!("Keyring session expired or needs unlocking after upgrade");
        }
    }

    let keyring = Keyring::new(SERVICE, fingerprint)?;
    let password = keyring.get_password()?;
    Ok(password)
}

pub fn clear_password(fingerprint: &str) -> KeyringResult<()> {
    let keyring = Keyring::new(SERVICE, fingerprint)?;
    state()?
        .set_session_expiry(fingerprint, None)
        .map_err(|error| keyring::Error::PlatformFailure(error.into()))?;
    keyring.delete_credential()
}
