//! Transactional, account/server scoped operational state. No private keys or plaintext secrets.
use super::config::{Config, Project};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{de::DeserializeOwned, Serialize};
use std::{fs, path::Path, time::Duration};

const SCHEMA_VERSION: i64 = 1;

pub struct SettingsWriteGuard<'a> {
    _transaction: rusqlite::Transaction<'a>,
}

pub struct StateStore {
    conn: Connection,
    scope: String,
}

impl StateStore {
    pub fn open(config: &Config) -> Result<Self> {
        let dir = home::home_dir()
            .context("Failed to get home directory")?
            .join(".config/envx");
        Self::open_at(&dir, config)
    }

    pub fn open_at(dir: &Path, config: &Config) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let path = dir.join("state.sqlite");
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(10))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        // A reserved writer lock serializes schema upgrades and one-time imports.
        let tx =
            conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 =
            tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            bail!("State database schema {version} is newer than this envx supports ({SCHEMA_VERSION}); upgrade envx");
        }
        if version == 0 {
            tx.execute_batch("CREATE TABLE records(scope TEXT NOT NULL, namespace TEXT NOT NULL, key TEXT NOT NULL, value BLOB NOT NULL, PRIMARY KEY(scope,namespace,key)); CREATE TABLE migrations(name TEXT PRIMARY KEY);")?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        let scope = Self::scope(config)?;
        let imported = tx
            .query_row(
                "SELECT 1 FROM migrations WHERE name='legacy-json-v1'",
                [],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !imported {
            // Keep exact pre-upgrade bytes before any later settings writes.
            if let Ok(bytes) = fs::read(dir.join("config.json")) {
                let backup = dir.join("config.pre-sqlite.json");
                let mut opts = fs::OpenOptions::new();
                opts.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    opts.mode(0o600);
                }
                let temporary = dir.join(format!(
                    "config.backup.{}.{}",
                    std::process::id(),
                    rand::random::<u64>()
                ));
                let preserved = (|| -> Result<()> {
                    use std::io::Write;
                    let mut file = opts.open(&temporary)?;
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                    match fs::hard_link(&temporary, backup) {
                        Ok(()) => Ok(()),
                        Err(e)
                            if e.kind()
                                == std::io::ErrorKind::AlreadyExists =>
                        {
                            Ok(())
                        }
                        Err(e) => Err(e.into()),
                    }
                })();
                let _ = fs::remove_file(temporary);
                preserved?;
            }
            // Legacy links belonged to the configured server and key. DEV_MODE must
            // never silently claim production links for localhost.
            let mut legacy_url = url::Url::parse(
                config.sdk_url.as_deref().unwrap_or("https://api.envx.sh"),
            )?;
            legacy_url.set_fragment(None);
            let legacy_scope = Self::scope_for(&legacy_url, config);
            for project in &config.projects {
                tx.execute(
                    "INSERT OR IGNORE INTO records VALUES(?1,'projects',?2,?3)",
                    params![
                        legacy_scope,
                        project
                            .path
                            .to_str()
                            .context("Project path is not UTF-8")?,
                        serde_json::to_vec(project)?
                    ],
                )?;
            }
            // This metadata is disposable. A damaged file must not block upgrading.
            if let Ok(bytes) = fs::read(dir.join("version.json")) {
                if let Ok(value) = serde_json::from_slice::<
                    crate::commands::update::UpdateCheck,
                >(&bytes)
                {
                    tx.execute("INSERT OR IGNORE INTO records VALUES('global','updates','latest',?1)", [serde_json::to_vec(&value)?])?;
                }
            }
            tx.execute("INSERT INTO migrations VALUES('legacy-json-v1')", [])?;
        }
        tx.commit()?;
        Ok(Self { conn, scope })
    }

    fn scope_for(url: &url::Url, config: &Config) -> String {
        let fingerprint = config
            .primary_key
            .as_ref()
            .map(|k| k.fingerprint.to_lowercase())
            .unwrap_or_else(|| "anonymous".into());
        let account = config
            .primary_key
            .as_ref()
            .and_then(|key| key.uuid.as_deref())
            .unwrap_or("unregistered");
        format!(
            "{}|{}|{}",
            url.as_str().trim_end_matches('/'),
            account,
            fingerprint
        )
    }

    fn scope(config: &Config) -> Result<String> {
        let mut url = config.sdk_url()?;
        url.set_fragment(None);
        Ok(Self::scope_for(&url, config))
    }

    pub fn lock_settings(&mut self) -> Result<SettingsWriteGuard<'_>> {
        Ok(SettingsWriteGuard {
            _transaction: self
                .conn
                .transaction_with_behavior(TransactionBehavior::Immediate)?,
        })
    }

    /// Run a read/validate/write sequence atomically. Nested transactions are rejected.
    pub fn with_write_lock<T>(
        &self,
        operation: impl FnOnce(&Self) -> Result<T>,
    ) -> Result<T> {
        let transaction = rusqlite::Transaction::new_unchecked(
            &self.conn,
            TransactionBehavior::Immediate,
        )?;
        let value = operation(self)?;
        transaction.commit()?;
        Ok(value)
    }

    pub fn get<T: DeserializeOwned>(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<T>> {
        self.get_bytes(namespace, key)?
            .map(|bytes| {
                serde_json::from_slice(&bytes).context("Invalid state record")
            })
            .transpose()
    }

    pub fn put<T: Serialize>(
        &self,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> Result<()> {
        self.put_bytes(namespace, key, &serde_json::to_vec(value)?)
    }

    pub fn put_if_absent<T: Serialize>(
        &self,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> Result<bool> {
        Ok(self.conn.execute(
            "INSERT OR IGNORE INTO records VALUES(?1,?2,?3,?4)",
            params![self.scope, namespace, key, serde_json::to_vec(value)?],
        )? == 1)
    }

    pub fn get_bytes(
        &self,
        namespace: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self.conn.query_row("SELECT value FROM records WHERE scope=?1 AND namespace=?2 AND key=?3", params![self.scope, namespace, key], |r| r.get(0)).optional()?)
    }

    pub fn put_bytes(
        &self,
        namespace: &str,
        key: &str,
        value: &[u8],
    ) -> Result<()> {
        self.conn.execute("INSERT INTO records VALUES(?1,?2,?3,?4) ON CONFLICT(scope,namespace,key) DO UPDATE SET value=excluded.value", params![self.scope, namespace, key, value])?;
        Ok(())
    }

    pub fn delete(&self, namespace: &str, key: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM records WHERE scope=?1 AND namespace=?2 AND key=?3",
            params![self.scope, namespace, key],
        )?;
        Ok(())
    }

    pub fn clear(&self, namespace: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM records WHERE scope=?1 AND namespace=?2",
            params![self.scope, namespace],
        )?;
        Ok(())
    }

    pub fn list<T: DeserializeOwned>(
        &self,
        namespace: &str,
    ) -> Result<Vec<(String, T)>> {
        let mut stmt = self.conn.prepare("SELECT key,value FROM records WHERE scope=?1 AND namespace=?2 ORDER BY key")?;
        let rows = stmt.query_map(params![self.scope, namespace], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
        })?;
        rows.map(|row| {
            let (key, bytes) = row?;
            Ok((key, serde_json::from_slice(&bytes)?))
        })
        .collect()
    }

    pub fn import_projects(
        &mut self,
        name: &str,
        projects: &[Project],
    ) -> Result<()> {
        let marker = format!("{}|{}", self.scope, name);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx
            .query_row(
                "SELECT 1 FROM migrations WHERE name=?1",
                [&marker],
                |_| Ok(()),
            )
            .optional()?
            .is_none()
        {
            for project in projects {
                tx.execute(
                    "INSERT OR IGNORE INTO records VALUES(?1,'projects',?2,?3)",
                    params![
                        self.scope,
                        project
                            .path
                            .to_str()
                            .context("Project path is not UTF-8")?,
                        serde_json::to_vec(project)?
                    ],
                )?;
            }
            tx.execute("INSERT INTO migrations VALUES(?1)", [marker])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn projects(&self) -> Result<Vec<Project>> {
        Ok(self
            .list("projects")?
            .into_iter()
            .map(|(_, value)| value)
            .collect())
    }

    pub fn session_expiry(
        &self,
        fingerprint: &str,
    ) -> Result<Option<std::time::SystemTime>> {
        let bytes: Option<Vec<u8>> = self.conn.query_row("SELECT value FROM records WHERE scope='global' AND namespace='keyring-expiry' AND key=?1", [fingerprint], |r| r.get(0)).optional()?;
        bytes
            .map(|bytes| {
                serde_json::from_slice(&bytes).context("Invalid keyring expiry")
            })
            .transpose()
    }

    pub fn set_session_expiry(
        &self,
        fingerprint: &str,
        expiry: Option<std::time::SystemTime>,
    ) -> Result<()> {
        if let Some(expiry) = expiry {
            self.conn.execute("INSERT INTO records VALUES('global','keyring-expiry',?1,?2) ON CONFLICT(scope,namespace,key) DO UPDATE SET value=excluded.value", params![fingerprint, serde_json::to_vec(&expiry)?])?;
        } else {
            self.conn.execute("DELETE FROM records WHERE scope='global' AND namespace='keyring-expiry' AND key=?1", [fingerprint])?;
        }
        Ok(())
    }

    pub fn update_check(&self) -> Result<crate::commands::update::UpdateCheck> {
        let bytes: Option<Vec<u8>> = self.conn.query_row("SELECT value FROM records WHERE scope='global' AND namespace='updates' AND key='latest'", [], |r| r.get(0)).optional()?;
        Ok(bytes
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default())
    }

    pub fn save_update_check(
        &self,
        value: &crate::commands::update::UpdateCheck,
    ) -> Result<()> {
        self.conn.execute("INSERT INTO records VALUES('global','updates','latest',?1) ON CONFLICT(scope,namespace,key) DO UPDATE SET value=excluded.value", [serde_json::to_vec(value)?])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_records_and_atomic_first_writer() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let store = StateStore::open_at(dir.path(), &config).unwrap();
        assert!(store.put_if_absent("pins", "friend", &"first").unwrap());
        assert!(!store.put_if_absent("pins", "friend", &"changed").unwrap());
        assert_eq!(
            store.get::<String>("pins", "friend").unwrap().unwrap(),
            "first"
        );
        let mut other = Config::default();
        other.sdk_url = Some("https://other.example".into());
        assert!(StateStore::open_at(dir.path(), &other)
            .unwrap()
            .get::<String>("pins", "friend")
            .unwrap()
            .is_none());
        other.sdk_url = Some("https://API.ENVX.SH:443/".into());
        assert!(StateStore::open_at(dir.path(), &other)
            .unwrap()
            .get::<String>("pins", "friend")
            .unwrap()
            .is_some());
    }

    #[test]
    fn account_scope_isolates_uuid_and_key_changes() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.primary_key = Some(super::super::key::Key {
            fingerprint: "ABC123".into(),
            note: "test".into(),
            primary_user_id: "test".into(),
            pubkey_only: None,
            uuid: None,
        });
        let first = StateStore::open_at(dir.path(), &config).unwrap();
        first.put("pins", "friend", &"trusted").unwrap();
        config.primary_key.as_mut().unwrap().uuid =
            Some("registered-user".into());
        assert!(StateStore::open_at(dir.path(), &config)
            .unwrap()
            .get::<String>("pins", "friend")
            .unwrap()
            .is_none());
        config.primary_key.as_mut().unwrap().fingerprint = "different".into();
        assert!(StateStore::open_at(dir.path(), &config)
            .unwrap()
            .get::<String>("pins", "friend")
            .unwrap()
            .is_none());
        let expiry = std::time::SystemTime::now();
        first.set_session_expiry("ABC123", Some(expiry)).unwrap();
        assert_eq!(first.session_expiry("ABC123").unwrap(), Some(expiry));
        first.set_session_expiry("ABC123", None).unwrap();
        assert!(first.session_expiry("ABC123").unwrap().is_none());
    }

    #[test]
    fn atomic_multi_record_operations_rollback_and_serialize() {
        let dir = tempfile::tempdir().unwrap();
        let store =
            StateStore::open_at(dir.path(), &Config::default()).unwrap();
        let result: Result<()> = store.with_write_lock(|store| {
            store.put("pins", "friend", &"partial")?;
            bail!("validation failed");
        });
        assert!(result.is_err());
        assert!(store.get::<String>("pins", "friend").unwrap().is_none());
        let mut workers = Vec::new();
        for fingerprint in ["first", "second"] {
            let path = dir.path().to_owned();
            workers.push(std::thread::spawn(move || {
                let store =
                    StateStore::open_at(&path, &Config::default()).unwrap();
                store
                    .with_write_lock(|store| {
                        if store.get::<String>("pins", "friend")?.is_some() {
                            return Ok(false);
                        }
                        std::thread::sleep(Duration::from_millis(20));
                        store.put("pins", "friend", &fingerprint)?;
                        store.put("pin-history", "friend", &fingerprint)?;
                        Ok(true)
                    })
                    .unwrap()
            }));
        }
        assert_eq!(
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap() as usize)
                .sum::<usize>(),
            1
        );
        assert_eq!(
            store.get::<String>("pins", "friend").unwrap(),
            store.get::<String>("pin-history", "friend").unwrap()
        );
    }

    #[test]
    fn failed_full_disk_write_preserves_previous_record() {
        let dir = tempfile::tempdir().unwrap();
        let store =
            StateStore::open_at(dir.path(), &Config::default()).unwrap();
        store.put("pins", "friend", &"original").unwrap();
        let pages: i64 = store
            .conn
            .pragma_query_value(None, "page_count", |r| r.get(0))
            .unwrap();
        store
            .conn
            .pragma_update(None, "max_page_count", pages)
            .unwrap();
        assert!(store.put("pins", "friend", &"x".repeat(2_000_000)).is_err());
        assert_eq!(
            store.get::<String>("pins", "friend").unwrap().unwrap(),
            "original"
        );
        let integrity: String = store
            .conn
            .pragma_query_value(None, "integrity_check", |r| r.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
    }

    #[test]
    fn future_schema_is_rejected_without_mutating_records() {
        let dir = tempfile::tempdir().unwrap();
        let store =
            StateStore::open_at(dir.path(), &Config::default()).unwrap();
        store.put("pins", "friend", &"original").unwrap();
        store.conn.pragma_update(None, "user_version", 999).unwrap();
        assert!(StateStore::open_at(dir.path(), &Config::default()).is_err());
        assert_eq!(
            store.get::<String>("pins", "friend").unwrap().unwrap(),
            "original"
        );
    }
}
