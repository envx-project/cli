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
            projects: vec![],
        }
    }
}

impl Config {
    // pub fn write(&self) -> Result<()> {
    //     write_config(self)?;
    //
    //     Ok(())
    // }

    /// Vulnerable to fs race conditions
    /// should rewrite using file locks
    pub fn write(&self) -> Result<()> {
        let path = get_config_path().context("Failed to get config path")?;
        let file = File::create(path).context("Failed to create config file")?;
        let mut writer = BufWriter::new(file);
        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize config to JSON string")?;
        writer
            .write_all(contents.as_bytes())
            .context("Failed to write config to file")?;

        Ok(())
    }

    #[allow(dead_code)]
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
            .context("Failed to find key (get_key)")?;

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
            .find(|k| {
                k.fingerprint
                    .to_lowercase()
                    .contains(&partial_fingerprint.to_lowercase())
            })
            .context("Failed to find key (get_key_or_default)")?;

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

    pub fn unset_project(&mut self) -> Result<()> {
        let path = std::env::current_dir()?;
        self.projects.retain(|p| p.path != path);
        self.write()?;

        Ok(())
    }

    pub fn set_uuid(&mut self, fingerprint: &str, uuid: &str) -> Result<()> {
        let mut key = Self::get_key(self, fingerprint)?;
        key.uuid = Some(uuid.to_string());
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
        let default = serde_json::to_string_pretty(&Config::default())?;
        fs::create_dir_all(path.parent().unwrap())?;
        let mut file = File::create(&path)?;
        file.write_all(default.as_ref())?;
    }
    Ok(path)
}

/// Read the configuration file and parse it into a Config struct
pub fn get_config() -> Result<Config> {
    let path = get_config_path().context("Failed to get config path")?;
    let contents = fs::read_to_string(path).context("Failed to read config file")?;
    serde_json::from_str::<Config>(&contents).context("Failed to parse config file")
}
