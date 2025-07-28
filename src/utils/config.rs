// configuration path = ~/.config/envx/config.json

use crate::utils::keyring::{get_password, set_password};
use crate::utils::prompt::prompt_password;

use super::compare_semver;
use super::key::Key;
use super::settings::Settings;
use anyhow::anyhow;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, IsTerminal, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    /// Last time the config was updated
    pub last_update_check: Option<DateTime<Utc>>,
    /// New version available
    pub new_version_available: Option<String>,
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
            last_update_check: None,
            new_version_available: None,
        }
    }
}

const GITHUB_API_RELEASE_URL: &'static str =
    "https://api.github.com/repos/envx-project/cli/releases/latest";

#[derive(Deserialize)]
struct GithubApiRelease {
    tag_name: String,
}

use once_cell::sync::Lazy;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
static CONFIG: Lazy<RwLock<Config>> =
    Lazy::new(|| RwLock::new(priv_get().unwrap()));

fn priv_get() -> Result<Config> {
    let path = get_config_file_path().context("Failed to get config path")?;
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

impl Config {
    pub fn try_get() -> Result<RwLockReadGuard<'static, Self>> {
        CONFIG.try_read().context("Failed to get config")
    }

    // allowed in case we need to synchronously write to the config
    #[allow(dead_code)]
    pub fn try_get_mut() -> Result<RwLockWriteGuard<'static, Self>> {
        CONFIG.try_write().context("Failed to get config")
    }

    pub async fn get() -> RwLockReadGuard<'static, Self> {
        CONFIG.read().await
    }

    pub async fn get_mut() -> RwLockWriteGuard<'static, Self> {
        // Note to future confused self: You may use the following code to panic at the exact place
        // where the config gets deadlocked. FML.
        // CONFIG
        //     .try_write()
        //     .context("Another Read (or Write) lock of config is held")
        //     .unwrap()
        CONFIG.write().await
    }

    // takes 700ms for some reason
    pub async fn check_update(
        &self,
        // &mut self,
        force: bool,
    ) -> anyhow::Result<Option<String>> {
        // outputting would break json output on CI
        if !std::io::stdout().is_terminal() && !force {
            return Ok(None);
        }

        if let Some(last_update_check) = self.last_update_check {
            if Utc::now().date_naive() == last_update_check.date_naive()
                && !force
            {
                return Ok(None);
            }
        }

        let client = reqwest::Client::new();
        let response = client
            .get(GITHUB_API_RELEASE_URL)
            .header("User-Agent", "envx")
            .send()
            .await?;

        let response = response.json::<GithubApiRelease>().await?;
        let latest_version = response.tag_name.trim_start_matches('v');

        match compare_semver(env!("CARGO_PKG_VERSION"), &latest_version) {
            Ordering::Less => Ok(Some(latest_version.to_owned())),
            _ => Ok(None),
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
