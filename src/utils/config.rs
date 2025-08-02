// configuration path = ~/.config/envx/config.json

use crate::utils::keyring::{get_password, set_password};
use crate::utils::prompt::prompt_password;

use super::key::{Key, UnlockedKey};
use super::settings::Settings;
use anyhow::anyhow;
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use envx_sdk::apis::configuration::Configuration;
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// TODO: rethink Salting hashes
    pub salt: String,
    /// The fingerprint of the primary signing key
    pub primary_key: Option<Key>,
    /// A vector of fingerprints of all usable public keys
    pub keys: Vec<Key>,
    /// Custom URL for the SDK
    pub sdk_url: Option<String>,
    /// Settings that apply to all environments
    pub settings: Option<Settings>,
    /// Projects
    pub projects: Vec<Project>,
    /// Password for the primary key
    pub primary_key_password: Option<String>,
}

impl Drop for Config {
    fn drop(&mut self) {
        if std::env::var("ENVX_DEBUG").is_ok() {
            dbg!("writing config");
        }
        self.write().unwrap();
    }
}

// TODO: add project name
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub project_id: String,
    pub path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let salt = hex::encode(rand::random::<[u8; 32]>());
        Self {
            salt,
            primary_key: None,
            keys: vec![],
            sdk_url: Some("https://api.envx.sh".into()),
            settings: None,
            projects: vec![],
            primary_key_password: None,
        }
    }
}

impl Config {
    pub fn get() -> Self {
        Config::priv_get().unwrap()
    }

    pub fn sdk_url(&self) -> Result<Url> {
        let dev_mode = std::env::var("DEV_MODE").is_ok();
        if dev_mode {
            return Ok(Url::parse("http://localhost:3000")?);
        }
        let url = Config::get()
            .sdk_url
            .clone()
            .unwrap_or("https://api.envx.sh".into());
        let url = Url::parse(&url)?;
        Ok(url)
    }

    pub fn sdk_configuration(
        &self,
        key: &UnlockedKey,
    ) -> Result<Configuration> {
        let mut configuration = Configuration::new();
        configuration.base_path = self
            .sdk_url()?
            .to_string()
            .trim_end_matches('/')
            .to_string();
        configuration.bearer_access_token = Some(key.auth_token()?.to_string());
        Ok(configuration)
    }

    fn priv_get() -> Result<Self> {
        let path =
            get_config_file_path().context("Failed to get config path")?;
        let contents =
            fs::read_to_string(path).context("Failed to read config file")?;

        let out = serde_json::from_str::<Config>(&contents)
            .context("Failed to parse config file");

        match out {
            Ok(c) => Ok(c),
            Err(e) => {
                if std::env::var("ENVX_DEBUG").is_ok() {
                    println!("Failed to parse config file: {}", e);
                    println!("Contents: {}", contents);
                };
                Err(e)
            }
        }
    }
    // NEVER call this function EVER
    pub fn write(&self) -> Result<()> {
        Config::priv_write(self)
    }

    fn priv_write<T>(value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let path =
            get_config_file_path().context("Failed to get config path")?;

        // Use the same directory for the temp file, so the rename is atomic
        let mut temp_path = path.clone();
        let nanos = Utc::now().timestamp_nanos_opt().unwrap();
        let pid = std::process::id();
        temp_path.set_extension(format!("tmp.{}-{}.json", pid, nanos));

        // Serialize to JSON
        let contents = serde_json::to_string_pretty(value)
            .context("Failed to serialize config to JSON string")?;

        // Write to the temp file
        let file = OpenOptions::new()
            .write(true)
            // atomically create tmp file, panic if it already exists (should never happen)
            .create_new(true)
            .truncate(true)
            .open(&temp_path)
            .context("Failed to create temp config file")?;
        let mut writer = BufWriter::new(file);

        writer
            .write_all(contents.as_bytes())
            .context("Failed to write config to temp file")?;
        writer.flush().context("Failed to flush writer")?;

        // Ensure data is physically written to disk
        writer
            .get_ref()
            .sync_all()
            .context("Failed to sync temp file to disk")?;

        // Atomically replace the old file
        fs::rename(&temp_path, &path)
            .context("Failed to atomically rename temp file")?;

        Ok(())
    }

    pub fn primary_key(&self) -> Result<Key> {
        self.primary_key.clone().context("No primary key set")
    }

    pub fn get_settings(&self) -> Settings {
        let settings = self.settings.clone();
        if let Some(settings) = settings {
            settings
        } else {
            Settings::default()
        }
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

    pub fn link_project(&mut self, project_id: &str) -> Result<()> {
        let path = std::env::current_dir()?;
        let new_project = Project {
            project_id: project_id.to_string(),
            path,
        };

        self.projects.push(new_project);
        Ok(())
    }

    pub fn unlink_project(&mut self) -> Result<Vec<String>> {
        let path = std::env::current_dir()?;
        let matching = self
            .projects
            .iter()
            .filter(|p| p.path == path)
            .map(|p| p.project_id.clone())
            .collect::<Vec<String>>();

        if matching.is_empty() {
            return Err(anyhow!("No project set in this directory".red()));
        }

        self.projects.retain(|p| p.path != path);
        Ok(matching)
    }

    pub fn delete_project(&mut self, project_id: &str) -> Result<()> {
        if self.projects.is_empty() {
            return Err(anyhow!("No projects to delete".red()));
        }
        if !self.projects.iter().any(|p| p.project_id == *project_id) {
            return Err(anyhow!("Project not found".red()));
        }
        self.projects.retain(|p| p.project_id != project_id);
        Ok(())
    }

    pub fn set_uuid(&mut self, fingerprint: &str, uuid: &str) -> Result<()> {
        for k in self.keys.iter_mut() {
            if k.fingerprint == fingerprint {
                k.uuid = Some(uuid.to_string());
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn primary_key_password(&self) -> Result<String> {
        let key = self.primary_key()?;

        let password = get_password(&self);
        match password {
            Ok(p) => Ok(p),
            Err(e) => {
                eprintln!("Failed to get password: {}", e);
                println!("Enter password for key {}", key);
                let password = prompt_password("Password: ")?;
                let expiry = self.get_settings().get_keyring_expiry();
                if let Err(e) =
                    set_password(&key.fingerprint, &password, expiry)
                {
                    eprintln!("Failed to set password: {}", e);
                }

                Ok(password)
            }
        }
    }
}

/// Get the configuration path ~/.config/envx/config.json
pub fn get_config_file_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;
    path.push(".config/envx/config.json");
    // if it doesn't exist, create it
    if !path.exists() {
        let default = serde_json::to_string_pretty(&Config::default())?;
        let parent_path =
            path.parent().context("Failed to get parent directory")?;
        fs::create_dir_all(parent_path)?;
        let mut file = File::create(&path)?;
        file.write_all(default.as_ref())?;
    }
    Ok(path)
}
