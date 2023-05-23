// configuration path = ~/.config/envcli/gpgconf.json
// make a reader and writer that uses file locks

use anyhow::{Context, Result};
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Key {
    pub name: String,
    pub email: String,
    pub number: i32,
    // pub passphrase: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub current_key: Key,
    pub keys: Vec<Key>,
}

#[allow(dead_code)]
pub fn get_config_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;
    path.push(".config");
    path.push("envcli");
    path.push("gpgconf.json");

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

#[allow(dead_code)]
pub fn get_config() -> Result<Config> {
    let path = get_config_path()?;
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    let config = serde_json::from_str::<Config>(&contents)?;

    Ok(config)
}

/// Vulnerable to fs race conditions
/// should rewrite using file locks
#[allow(dead_code)]
pub fn write_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let contents = serde_json::to_string_pretty(config)?;
    writer.write_all(contents.as_bytes())?;

    Ok(())
}
