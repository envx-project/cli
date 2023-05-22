// configuration path = ~/.config/envcli/config.json
// make a reader and writer that uses file locks

use anyhow::{Context, Result};
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub user_id: String,
    pub password: String,
}

pub fn get_config_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;

    path.push(".config");
    path.push("envcli");
    path.push("config.json");
    // if it doesn't exist, create it
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap())?;
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all("".as_bytes())?;
    }
    Ok(path)
}

pub fn get_config() -> Result<Config> {
    let path = get_config_path()?;
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    let config = serde_json::from_str::<Config>(&contents)?;

    Ok(config)
}

pub fn write_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let contents = serde_json::to_string_pretty(config)?;
    writer.write_all(contents.as_bytes())?;
    Ok(())
}

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
