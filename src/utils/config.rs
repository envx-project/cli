// configuration path = ~/.config/envcli/config.json

use super::key::Key;
use super::rpgp::get_vault_location;
use super::settings::Settings;
use anyhow::{Context, Result};
use colored::Colorize;
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub salt: String,
    /// The fingerprint of the primary signing key
    pub primary_key: String,
    /// A vector of fingerprints of all usable public keys
    pub keys: Vec<Key>,
    /// Use the SDK or not
    pub online: bool,
    /// Custom URL for the SDK
    pub sdk_url: Option<String>,
    /// Settings that apply to all environments
    pub settings: Option<Settings>,
    /// Default project ID
    pub default_project_id: Option<String>,
    /// Silence startup message
    pub silent: Option<bool>,
}

impl Config {
    pub fn write(&self, global: bool) -> Result<()> {
        if global {
            write_config(self)?;
        } else {
            write_local_config(self)?;
        }
        Ok(())
    }

    pub fn primary_key(&self) -> Result<String> {
        let primary_key = self.primary_key.clone();
        let primary_key_location = get_vault_location()?
            .join(primary_key.clone())
            .join("public.key");

        let primary_public_key = fs::read_to_string(primary_key_location)
            .context("Failed to read primary public key")?;

        Ok(primary_public_key)
    }

    pub fn add_key(&self, key: Key) -> Result<Config> {
        unimplemented!()
    }

    pub fn get_settings(&self) -> Result<Settings> {
        let settings = self.settings.clone();
        if let Some(settings) = settings {
            Ok(settings)
        } else {
            Ok(Settings::default())
        }
    }

    pub fn get_key(&self, partial_fingerprint: &str) -> Result<Key> {
        let key = self
            .keys
            .iter()
            .find(|k| k.fingerprint.contains(partial_fingerprint))
            .context("Failed to find key")?;

        Ok(key.clone())
    }

    pub fn set_project_id(&mut self, project_id: &str) -> Result<()> {
        self.default_project_id = Some(project_id.to_string());
        Ok(())
    }
}

pub fn get_specific_config(global: bool) -> Result<Config> {
    if global {
        get_config()
    } else {
        get_local_config()
    }
}

pub fn get_local_or_global_config() -> Result<Config> {
    let local_config = get_local_config();
    if local_config.is_ok() {
        return local_config;
    }

    get_config()
}

#[allow(dead_code)]
pub fn get_envcli_dir() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;
    path.push(".config");
    path.push("envcli");
    Ok(path)
}

/// Get the configuration path ~/.config/envcli/config.json
pub fn get_config_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;
    path.push(".config/envcli/config.json");
    // if it doesn't exist, create it
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap())?;
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all("".as_bytes())?;
    }
    Ok(path)
}

/// Read the configuration file and parse it into a Config struct
pub fn get_config() -> Result<Config> {
    let path = get_config_path()?;
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str::<Config>(&contents).context("Failed to parse config file")?)
}

/// Vulnerable to fs race conditions
/// should rewrite using file locks
pub fn write_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    let file = File::create(path)?;

    let mut writer = BufWriter::new(file);
    let contents = serde_json::to_string_pretty(config)?;
    writer.write_all(contents.as_bytes())?;

    Ok(())
}

/// Get the local configuration path .envcli.json
#[allow(dead_code)]
pub fn get_local_config() -> Result<Config, anyhow::Error> {
    let mut path = std::env::current_dir()?;
    loop {
        let file_path = path.join(".envcli.json");
        if file_path.exists() {
            let file = File::open(file_path)?;
            let mut reader = BufReader::new(file);
            let mut contents = String::new();
            reader.read_to_string(&mut contents)?;
            let config =
                serde_json::from_str::<Config>(&contents).context("Failed to parse config file")?;

            return Ok(config);
        }

        if !path.pop() {
            return Err(anyhow::anyhow!(
                "No .envcli.json found.\nTry `envcli init`".bright_red()
            ));
        }
    }
}

/// Write the local configuration file .envcli.json
pub fn write_local_config(config: &Config) -> Result<()> {
    let mut path = std::env::current_dir()?;
    path.push(".envcli.json");

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let contents = serde_json::to_string_pretty(config)?;
    writer.write_all(contents.as_bytes())?;

    Ok(())
}
