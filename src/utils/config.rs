// configuration path = ~/.config/envx/config.json

use crate::utils::keyring::{get_password, set_password};
use crate::utils::prompt::prompt_password;

use super::key::{Key, UnlockedKey};
use super::settings::Settings;
use anyhow::anyhow;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use colored::Colorize;
use envx_sdk::apis::configuration::Configuration;
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// TODO: rethink Salting hashes
    pub salt: String,
    #[serde(skip)]
    original: Option<serde_json::Value>,
    /// The fingerprint of the primary signing key
    pub primary_key: Option<Key>,
    /// Custom URL for the SDK
    pub sdk_url: Option<String>,
    /// Settings that apply to all environments
    pub settings: Option<Settings>,
    /// Projects
    #[serde(default)]
    pub projects: Vec<Project>,
    /// Password for the primary key
    pub primary_key_password: Option<String>,
    /// Command to run to get the primary key
    pub primary_key_command: Option<Vec<String>>,
}

// impl Drop for Config {
//     fn drop(&mut self) {
//         if std::env::var("ENVX_DEBUG").is_ok() {
//             dbg!("writing config");
//         }
//         match self.write() {
//             Ok(_) => {}
//             Err(e) => {
//                 eprintln!("Failed to write config: {}", e);
//             }
//         }
//     }
// }
//
// TODO: add project name
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Project {
    // TODO: make this a UUID
    pub project_id: String,
    pub path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let salt = hex::encode(rand::random::<[u8; 32]>());
        Self {
            salt,
            original: None,
            primary_key: None,
            sdk_url: Some("https://api.envx.sh".into()),
            settings: None,
            projects: vec![],
            primary_key_password: None,
            primary_key_command: None,
        }
    }
}

impl Config {
    pub fn get() -> Self {
        Config::load().unwrap()
    }

    pub fn sdk_url(&self) -> Result<Url> {
        let dev_mode = std::env::var("DEV_MODE").is_ok();
        if dev_mode {
            return Ok(Url::parse("http://localhost:3000")?);
        }
        let url = self.sdk_url.clone().unwrap_or("https://api.envx.sh".into());
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

    pub fn load() -> Result<Self> {
        let path =
            get_config_file_path().context("Failed to get config path")?;
        let contents =
            fs::read_to_string(path).context("Failed to read config file")?;

        let mut config = Self::decode(&contents)?;
        config.original = Some(serde_json::to_value(&config)?);
        config.projects =
            super::state::StateStore::open(&config)?.projects()?;
        Ok(config)
    }

    pub fn decode(contents: &str) -> Result<Self> {
        let mut value: serde_json::Value = serde_json::from_str(contents)
            .context("Failed to parse config file")?;
        // v2.0 stored a fingerprint plus a keys array; later releases store Key.
        if let Some(fingerprint) = value
            .get("primary_key")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
        {
            let key = if fingerprint.is_empty() {
                serde_json::Value::Null
            } else {
                value.get("keys").and_then(|v| v.as_array())
                    .and_then(|keys| keys.iter().find(|key| key.get("fingerprint").and_then(|v| v.as_str()) == Some(&fingerprint)))
                    .cloned().context("Legacy primary key is missing from keys; original config preserved")?
            };
            value["primary_key"] = key;
        }
        let config: Config = serde_json::from_value(value)
            .context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn apply_edited(
        &mut self,
        mut edited: Self,
        original: &str,
    ) -> Result<()> {
        let baseline = Self::decode(original)?;
        if edited.projects != baseline.projects {
            bail!("Project links are managed in SQLite; use `envx link` or `envx unlink` instead of editing projects in config.json");
        }
        // The editor opens the retained JSON snapshot, whose project list can
        // differ from current SQLite links. Keep those live links, and diff
        // settings against the exact buffer the user started editing.
        edited.original = Some(serde_json::to_value(&baseline)?);
        edited.projects = self.projects.clone();
        *self = edited;
        Ok(())
    }

    pub fn write(&mut self) -> Result<()> {
        let mut store = super::state::StateStore::open(self)?;
        let _guard = store.lock_settings()?;
        let path =
            get_config_file_path().context("Failed to get config path")?;

        // Use the same directory for the temp file, so the rename is atomic
        let mut temp_path = path.clone();
        let nanos = Utc::now().timestamp_nanos_opt().unwrap();
        let pid = std::process::id();
        temp_path.set_extension(format!("tmp.{}-{}.json", pid, nanos));

        // Serialize to JSON
        // Preserve unknown fields and the legacy projects snapshot for downgrade
        // recovery. Operational changes are only written through StateStore.
        let original = fs::read_to_string(&path)?;
        let mut merged: serde_json::Value = serde_json::from_str(&original)?;
        let mut current = serde_json::to_value(&*self)?;
        current
            .as_object_mut()
            .context("Invalid config")?
            .remove("projects");
        let mut baseline =
            self.original.clone().unwrap_or(serde_json::Value::Null);
        // Older profiles can omit the entire settings object. Compare against
        // effective defaults so independent first settings edits still merge.
        if baseline.get("settings").map_or(true, |v| v.is_null())
            && current.get("settings").is_some_and(|v| v.is_object())
        {
            baseline["settings"] = serde_json::to_value(Settings::default())?;
        }
        merge_changed_fields(&mut merged, &current, &baseline);
        if merged == serde_json::from_str::<serde_json::Value>(&original)? {
            return Ok(());
        }
        let contents = serde_json::to_string_pretty(&merged)?;

        // Write to the temp file
        let mut options = OpenOptions::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
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
        self.original = Some(serde_json::to_value(&*self)?);

        Ok(())
    }

    pub fn unlocked_primary_key(&self) -> Result<UnlockedKey> {
        let key = self.primary_key()?;
        let password = self.primary_key_password()?;
        Ok(key.unlock(&password))
    }

    pub fn primary_key(&self) -> Result<Key> {
        self.primary_key.clone().context("No primary key set")
    }

    pub fn get_settings(&self) -> Settings {
        self.settings.clone().unwrap_or_default()
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

        super::state::StateStore::open(self)?.put(
            "projects",
            new_project
                .path
                .to_str()
                .context("Project path is not UTF-8")?,
            &new_project,
        )?;
        self.projects.retain(|p| p.path != new_project.path);
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
            .collect::<Vec<_>>();

        if matching.is_empty() {
            return Err(anyhow!("No project set in this directory".red()));
        }

        super::state::StateStore::open(self)?.delete(
            "projects",
            path.to_str().context("Project path is not UTF-8")?,
        )?;
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
        let store = super::state::StateStore::open(self)?;
        for project in
            self.projects.iter().filter(|p| p.project_id == project_id)
        {
            store.delete(
                "projects",
                project.path.to_str().context("Project path is not UTF-8")?,
            )?;
        }
        self.projects.retain(|p| p.project_id != project_id);
        Ok(())
    }

    pub fn set_uuid(&mut self, fingerprint: &str, uuid: &str) -> Result<()> {
        if let Some(k) = self.primary_key.as_mut() {
            if k.fingerprint == fingerprint {
                k.uuid = Some(uuid.to_string());
            }
        }

        Ok(())
    }

    pub fn primary_key_password(&self) -> Result<String> {
        let key = self.primary_key()?;

        let password = get_password(self);
        match password {
            Ok(p) => Ok(p),
            Err(e) => {
                eprintln!("Failed to get password: {}", e);
                let mut last_error = None;
                for _ in 0..3 {
                    println!("Enter password for key {}", key);
                    let password = prompt_password("Password: ")?;

                    match key.verify_passphrase(&password) {
                        Ok(()) => {
                            let expiry =
                                self.get_settings().get_keyring_expiry();
                            if let Err(e) = set_password(
                                &key.fingerprint,
                                &password,
                                expiry,
                            ) {
                                eprintln!("Failed to set password: {}", e);
                            }

                            return Ok(password);
                        }
                        Err(e) => {
                            eprintln!("Invalid password: {}", e);
                            last_error = Some(e);
                        }
                    }
                }

                if let Some(e) = last_error {
                    bail!(
                        "failed to unlock primary key after 3 attempts: {}",
                        e
                    );
                }

                bail!("failed to unlock primary key after 3 attempts")
            }
        }
    }
}

fn merge_changed_fields(
    existing: &mut serde_json::Value,
    current: &serde_json::Value,
    baseline: &serde_json::Value,
) {
    if current == baseline {
        return;
    }
    if let Some(fields) = current.as_object() {
        if !existing.is_object() {
            *existing = if baseline.is_object() {
                baseline.clone()
            } else {
                serde_json::json!({})
            };
        }
        for (key, value) in fields {
            let old = baseline.get(key).unwrap_or(&serde_json::Value::Null);
            if old != value {
                merge_changed_fields(&mut existing[key], value, old);
            }
        }
    } else {
        *existing = current.clone();
    }
}

/// Get the configuration path ~/.config/envx/config.json
pub fn get_config_file_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Failed to get home directory")?;
    path.push(".config/envx/config.json");
    // if it doesn't exist, create it
    if !path.exists() {
        let tmp = std::mem::ManuallyDrop::new(Config::default());
        let default = serde_json::to_string_pretty(&*tmp)?;
        let parent_path =
            path.parent().context("Failed to get parent directory")?;
        fs::create_dir_all(parent_path)?;
        let temp = parent_path.join(format!(
            "config.init.{}.{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        file.write_all(default.as_ref())?;
        file.sync_all()?;
        let published = fs::hard_link(&temp, &path);
        fs::remove_file(&temp)?;
        match published {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(path)
}

#[cfg(test)]
mod state_tests {
    use super::*;

    #[test]
    fn editor_preserves_baseline_and_rejects_legacy_project_edits() {
        let mut config = Config::load().unwrap();
        let path = get_config_file_path().unwrap();
        let original = fs::read_to_string(&path).unwrap();
        let mut edited = Config::decode(&original).unwrap();
        edited.settings = Some(Settings {
            loud: Some(true),
            ..edited.get_settings()
        });
        let mut concurrent: serde_json::Value =
            serde_json::from_str(&original).unwrap();
        concurrent["sdk_url"] =
            serde_json::json!("https://editor-concurrent.example");
        fs::write(&path, serde_json::to_vec(&concurrent).unwrap()).unwrap();
        config.apply_edited(edited, &original).unwrap();
        config.write().unwrap();
        let loaded = Config::load().unwrap();
        assert_eq!(
            loaded.sdk_url.as_deref(),
            Some("https://editor-concurrent.example")
        );
        assert!(loaded.get_settings().is_loud());
        let mut edited = Config::decode(&original).unwrap();
        edited.projects.push(Project {
            project_id: "attempted-json-link".into(),
            path: PathBuf::from("/project"),
        });
        assert!(config.apply_edited(edited, &original).is_err());
    }

    #[test]
    fn concurrent_writer_worker() {
        let Ok(field) = std::env::var("ENVX_TEST_WRITE_FIELD") else {
            return;
        };
        let barrier =
            PathBuf::from(std::env::var("ENVX_TEST_BARRIER").unwrap());
        let mut config = Config::load().unwrap();
        let mut settings = config.get_settings();
        if field == "loud" {
            settings.loud = Some(true);
        } else {
            settings.warn_on_short_passwords = true;
        }
        config.settings = Some(settings);
        fs::write(barrier.join(&field), b"ready").unwrap();
        while !barrier.join("go").exists() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        config.write().unwrap();
    }

    #[test]
    fn simultaneous_process_settings_edits_merge() {
        let home = tempfile::tempdir().unwrap();
        let barrier = tempfile::tempdir().unwrap();
        let executable = std::env::current_exe().unwrap();
        let mut children = Vec::new();
        for field in ["loud", "warn"] {
            children.push(
                std::process::Command::new(&executable)
                    .args([
                        "--exact",
                        "utils::config::state_tests::concurrent_writer_worker",
                        "--test-threads=1",
                    ])
                    .env("HOME", home.path())
                    .env("ENVX_TEST_WRITE_FIELD", field)
                    .env("ENVX_TEST_BARRIER", barrier.path())
                    .spawn()
                    .unwrap(),
            );
        }
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !barrier.path().join("loud").exists()
            || !barrier.path().join("warn").exists()
        {
            assert!(
                std::time::Instant::now() < deadline,
                "writer failed to reach barrier"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        fs::write(barrier.path().join("go"), b"go").unwrap();
        for mut child in children {
            assert!(child.wait().unwrap().success());
        }
        let config = Config::decode(
            &fs::read_to_string(home.path().join(".config/envx/config.json"))
                .unwrap(),
        )
        .unwrap();
        assert!(config.get_settings().is_loud());
        assert!(config.get_settings().warn_on_short_passwords);
    }

    #[test]
    fn unchanged_loaded_config_does_not_overwrite_concurrent_settings() {
        let mut config = Config::load().unwrap();
        let path = get_config_file_path().unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["unknown_setting"] = serde_json::json!({"keep": true});
        value["sdk_url"] = serde_json::json!("https://changed.example");
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &bytes).unwrap();
        config.write().unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn settings_write_keeps_unknown_fields_and_private_permissions() {
        let mut config = Config::load().unwrap();
        config.primary_key_password = Some("synthetic-test-secret".into());
        config.write().unwrap();
        let path = get_config_file_path().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        config.primary_key_password = None;
        config.write().unwrap();
        assert!(Config::load().unwrap().primary_key_password.is_none());
    }
}
