// configuration path = ~/.config/envcli/config.json

use super::key::Key;
use super::rpgp::get_vault_location;
use super::settings::Settings;
use anyhow::{Context, Result};
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
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
    /// Silence startup message
    pub silent: Option<bool>,
    /// Projects
    pub projects: Vec<Project>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub project_id: String,
    pub path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            salt: "".into(),
            primary_key: "".into(),
            keys: vec![],
            online: false,
            sdk_url: None,
            settings: None,
            silent: None,
            projects: vec![],
        }
    }
}

impl Config {
    pub fn write(&self) -> Result<()> {
        write_config(self)?;

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

    // TODO: write an implementation to add key to config
    /// Add a key to the config and write it to disk
    #[allow(dead_code)]
    pub fn add_key(&self, _key: Key) -> Result<Config> {
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

    pub fn get_key_or_default(&self, partial_fingerprint: Option<String>) -> Result<Key> {
        let partial_fingerprint = match partial_fingerprint {
            Some(p) => p,
            None => self.primary_key.clone(),
        };
        if partial_fingerprint.is_empty() {
            return Err(anyhow::anyhow!("No key provided"));
        }

        let key = self
            .keys
            .iter()
            .find(|k| k.fingerprint.contains(&partial_fingerprint))
            .context("Failed to find key")?;

        Ok(key.clone())
    }

    #[allow(dead_code)]
    pub fn init_project(&mut self, project_id: &str, path: PathBuf) -> Result<()> {
        let project = Project {
            project_id: project_id.to_string(),
            path,
        };

        self.projects.push(project);
        self.write()?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_project(&self) -> Result<&Project> {
        let mut path = std::env::current_dir()?;
        loop {
            let project = self.projects.iter().find(|p| p.path == path);
            if let Some(project) = project {
                return Ok(project);
            }
            if !path.pop() {
                break;
            }
        }
        Err(anyhow::anyhow!("Failed to find project"))
    }

    pub fn set_project(&mut self, project_id: &str) -> Result<()> {
        let path = std::env::current_dir()?;
        let new_project = Project {
            project_id: project_id.to_string(),
            path,
        };

        self.projects.push(new_project);
        self.write()?;

        Ok(())
    }
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
