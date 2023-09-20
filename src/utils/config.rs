// configuration path = ~/.config/envcli/config.json
// make a reader and writer that uses file locks

use anyhow::{Context, Result};
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Key {
    pub fingerprint: String,
    pub note: String,
    pub primary_user_id: String,
    pub hashed_note: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub salt: String,
    /// The fingerprint of the primary signing key
    pub primary_key: String,
    /// A vector of fingerprints of all usable public keys
    pub keys: Vec<Key>,
}

pub fn get_envcli_dir() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;
    path.push(".config");
    path.push("envcli");
    Ok(path)
}

/// Get the configuration path ~/.config/envcli/config.json
pub fn get_config_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;
    path.push(".config");
    path.push("envcli");
    path.push("config.json");

    // if it doesn't exist, create it
    if !path.exists() {
        fs::create_dir_all(
            path.parent()
                .context("Failed to get parent directory")
                .unwrap(),
        )?;
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all("".as_bytes())?;
    }
    Ok(path)
}

/// Read the configuration file and parse it into a Config struct
pub fn get_config() -> Result<Config> {
    let path = get_config_path()?;
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    let config =
        serde_json::from_str::<Config>(&contents).context("Failed to parse config file")?;

    Ok(config)
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
pub fn local_get_config() -> Result<Config> {
    let mut path = std::env::current_dir()?;
    path.push(".envcli.json");

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    let config = serde_json::from_str::<Config>(&contents)?;

    Ok(config)
}

/// Write the local configuration file .envcli.json
#[allow(dead_code)]
pub fn write_local_config(config: &Config) -> Result<()> {
    let mut path = std::env::current_dir()?;
    path.push(".envcli.json");

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let contents = serde_json::to_string_pretty(config)?;
    writer.write_all(contents.as_bytes())?;

    Ok(())
}
