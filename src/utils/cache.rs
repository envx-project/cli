use std::fs;
use std::io::{BufReader, Cursor};
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use pgp::composed::{
    Message, MessageBuilder, SignedPublicKey, SignedSecretKey,
};
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::types::CompressionAlgorithm;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

use super::key::UnlockedKey;
use super::variable::DecryptedVariable;

#[derive(Serialize, Deserialize)]
struct CacheEnvelope {
    cached_at: DateTime<Utc>,
    variables: Vec<DecryptedVariable>,
}

fn cache_dir() -> Result<PathBuf> {
    let path = home::home_dir()
        .context("Failed to get home directory")?
        .join(".config")
        .join("envx")
        .join("cache");
    Ok(path)
}

fn cache_file_path(project_id: &str, project_name: &str) -> Result<PathBuf> {
    let sanitized_name = project_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(cache_dir()?.join(format!("{}_{}.envx", project_id, sanitized_name)))
}

pub fn write_cache(
    project_id: &str,
    project_name: &str,
    variables: &[DecryptedVariable],
    key: &UnlockedKey,
) -> Result<()> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir).context("Failed to create cache directory")?;

    let envelope = CacheEnvelope {
        cached_at: Utc::now(),
        variables: variables.to_vec(),
    };

    let json = serde_json::to_string(&envelope)
        .context("Failed to serialize cache envelope")?;

    let pubkey =
        SignedPublicKey::try_from(key).map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut rng = StdRng::from_entropy();
    let mut builder = MessageBuilder::from_bytes("", json.as_bytes().to_vec())
        .seipd_v1(&mut rng, SymmetricKeyAlgorithm::AES256);
    builder.compression(CompressionAlgorithm::ZLIB);
    builder.encrypt_to_key(&mut rng, &pubkey)?;

    let encrypted = builder.to_vec(&mut rng)?;

    let path = cache_file_path(project_id, project_name)?;

    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let pid = std::process::id();
    let tmp_path = path.with_extension(format!("tmp.{}-{}", pid, nanos));

    fs::write(&tmp_path, &encrypted)
        .context("Failed to write cache temp file")?;
    fs::rename(&tmp_path, &path)
        .context("Failed to atomically rename cache file")?;

    Ok(())
}

pub struct CachedVariables {
    pub variables: Vec<DecryptedVariable>,
    pub cached_at: DateTime<Utc>,
}

fn find_cache_file(project_id: &str) -> Result<PathBuf> {
    let dir = cache_dir()?;
    let prefix = format!("{}_", project_id);
    for entry in fs::read_dir(&dir).context("Cache directory not found")? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with(&prefix) && name.ends_with(".envx") {
                return Ok(entry.path());
            }
        }
    }
    anyhow::bail!("No cache file found for project {}", project_id)
}

pub fn read_cache(
    project_id: &str,
    project_name: Option<&str>,
    key: &UnlockedKey,
) -> Result<CachedVariables> {
    let path = match project_name {
        Some(name) => cache_file_path(project_id, name)?,
        None => find_cache_file(project_id)?,
    };
    let data = fs::read(&path).context("Failed to read cache file")?;

    let seckey =
        SignedSecretKey::try_from(key).map_err(|e| anyhow::anyhow!("{}", e))?;

    let msg = Message::from_bytes(BufReader::new(Cursor::new(data)))
        .context("Failed to parse cached PGP message")?;

    let mut decrypted = msg
        .decrypt(&key.password.clone().into(), &seckey)
        .context("Failed to decrypt cache")?;

    if decrypted.is_compressed() {
        decrypted = decrypted
            .decompress()
            .context("Failed to decompress cache")?;
    }

    let plaintext = decrypted.as_data_string()?;

    let envelope: CacheEnvelope = serde_json::from_str(&plaintext)
        .context("Failed to parse cache envelope JSON")?;

    Ok(CachedVariables {
        variables: envelope.variables,
        cached_at: envelope.cached_at,
    })
}

#[allow(dead_code)]
pub fn wipe_cache_for_project(project_id: &str) -> Result<()> {
    let dir = match cache_dir() {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };
    if !dir.exists() {
        return Ok(());
    }
    let prefix = format!("{}_", project_id);
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with(&prefix) && name.ends_with(".envx") {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

pub fn wipe_all_caches() -> Result<()> {
    let dir = match cache_dir() {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };
    if dir.exists() {
        fs::remove_dir_all(&dir).context("Failed to remove cache directory")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::key::Key;
    use crate::utils::rpgp::generate_key_pair;
    use pgp::types::KeyDetails;

    fn test_key() -> (UnlockedKey, Key) {
        let password = "test-password-123".to_string();
        let kp = generate_key_pair("test-cache", password.clone()).unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let fp_dir = tmp.path().join("TESTFP");
        fs::create_dir_all(&fp_dir).unwrap();

        let pub_armor = kp
            .public_key
            .to_armored_string(pgp::composed::ArmorOptions::default())
            .unwrap();
        let sec_armor = kp
            .secret_key
            .to_armored_string(pgp::composed::ArmorOptions::default())
            .unwrap();

        fs::write(fp_dir.join("public.key"), &pub_armor).unwrap();
        fs::write(fp_dir.join("private.key"), &sec_armor).unwrap();

        let fingerprint = hex::encode(kp.public_key.fingerprint());

        let key_dir = home::home_dir()
            .unwrap()
            .join(".config/envx/keys")
            .join(&fingerprint);
        fs::create_dir_all(&key_dir).unwrap();
        fs::write(key_dir.join("public.key"), &pub_armor).unwrap();
        fs::write(key_dir.join("private.key"), &sec_armor).unwrap();

        let key = Key {
            fingerprint: fingerprint.clone(),
            note: "test key".to_string(),
            primary_user_id: "test".to_string(),
            pubkey_only: None,
            uuid: Some("test-uuid".to_string()),
        };
        let unlocked = key.clone().unlock(&password);
        (unlocked, key)
    }

    fn make_vars() -> Vec<DecryptedVariable> {
        use crate::utils::kvpair::KVPair;
        vec![
            DecryptedVariable {
                id: "var-1".into(),
                value: KVPair::new(
                    "DATABASE_URL".into(),
                    "postgres://localhost/test".into(),
                ),
                project_id: "proj-1".into(),
                created_at: "2026-01-01T00:00:00Z".into(),
            },
            DecryptedVariable {
                id: "var-2".into(),
                value: KVPair::new("API_KEY".into(), "sk-secret-123".into()),
                project_id: "proj-1".into(),
                created_at: "2026-01-02T00:00:00Z".into(),
            },
        ]
    }

    #[test]
    fn round_trip_write_read() {
        let (key, _) = test_key();
        let vars = make_vars();

        write_cache("proj-1", "my-project", &vars, &key).unwrap();
        let cached = read_cache("proj-1", Some("my-project"), &key).unwrap();

        assert_eq!(cached.variables.len(), 2);
        assert_eq!(cached.variables[0].value.key, "DATABASE_URL");
        assert_eq!(cached.variables[1].value.key, "API_KEY");
        assert!(cached.cached_at <= Utc::now());

        // cleanup
        wipe_cache_for_project("proj-1").unwrap();
    }

    #[test]
    fn wipe_project_cache_only_removes_target() {
        let (key, _) = test_key();
        let vars = make_vars();

        write_cache("proj-1", "my-project", &vars, &key).unwrap();
        write_cache("proj-2", "other-project", &vars, &key).unwrap();

        wipe_cache_for_project("proj-1").unwrap();

        assert!(read_cache("proj-1", Some("my-project"), &key).is_err());
        assert!(read_cache("proj-2", Some("other-project"), &key).is_ok());

        wipe_cache_for_project("proj-2").unwrap();
    }

    #[test]
    fn wipe_all_removes_everything() {
        let (key, _) = test_key();
        let vars = make_vars();

        write_cache("proj-1", "my-project", &vars, &key).unwrap();
        write_cache("proj-2", "other-project", &vars, &key).unwrap();

        wipe_all_caches().unwrap();

        assert!(read_cache("proj-1", Some("my-project"), &key).is_err());
        assert!(read_cache("proj-2", Some("other-project"), &key).is_err());
    }

    #[test]
    fn read_nonexistent_cache_fails() {
        let (key, _) = test_key();
        assert!(read_cache("nonexistent", Some("fake"), &key).is_err());
    }

    #[test]
    fn wrong_key_cannot_decrypt() {
        let (key1, _) = test_key();
        let vars = make_vars();

        write_cache("proj-wrong-key", "test", &vars, &key1).unwrap();

        let password2 = "different-password".to_string();
        let kp2 = generate_key_pair("other-user", password2.clone()).unwrap();

        let fp2 = hex::encode(kp2.public_key.fingerprint());
        let key_dir2 = home::home_dir()
            .unwrap()
            .join(".config/envx/keys")
            .join(&fp2);
        fs::create_dir_all(&key_dir2).unwrap();

        let pub2 = kp2
            .public_key
            .to_armored_string(pgp::composed::ArmorOptions::default())
            .unwrap();
        let sec2 = kp2
            .secret_key
            .to_armored_string(pgp::composed::ArmorOptions::default())
            .unwrap();
        fs::write(key_dir2.join("public.key"), &pub2).unwrap();
        fs::write(key_dir2.join("private.key"), &sec2).unwrap();

        let key2 = Key {
            fingerprint: fp2.clone(),
            note: "other".into(),
            primary_user_id: "other".into(),
            pubkey_only: None,
            uuid: Some("other-uuid".into()),
        };
        let unlocked2 = key2.unlock(&password2);

        assert!(read_cache("proj-wrong-key", Some("test"), &unlocked2).is_err());

        // cleanup
        wipe_cache_for_project("proj-wrong-key").unwrap();
        let _ = fs::remove_dir_all(&key_dir2);
    }

    #[test]
    fn cache_file_naming() {
        let path = cache_file_path("abc-123", "My Cool Project").unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(name, "abc-123_My_Cool_Project.envx");
    }

    #[test]
    fn empty_variables_round_trip() {
        let (key, _) = test_key();
        let vars: Vec<DecryptedVariable> = vec![];

        write_cache("proj-empty", "empty", &vars, &key).unwrap();
        let cached = read_cache("proj-empty", Some("empty"), &key).unwrap();

        assert_eq!(cached.variables.len(), 0);

        wipe_cache_for_project("proj-empty").unwrap();
    }

    #[test]
    fn overwrite_cache() {
        let (key, _) = test_key();
        use crate::utils::kvpair::KVPair;

        let vars1 = vec![DecryptedVariable {
            id: "v1".into(),
            value: KVPair::new("OLD".into(), "old-value".into()),
            project_id: "proj-ow".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        }];

        write_cache("proj-ow", "overwrite-test", &vars1, &key).unwrap();

        let vars2 = vec![DecryptedVariable {
            id: "v2".into(),
            value: KVPair::new("NEW".into(), "new-value".into()),
            project_id: "proj-ow".into(),
            created_at: "2026-01-02T00:00:00Z".into(),
        }];

        write_cache("proj-ow", "overwrite-test", &vars2, &key).unwrap();

        let cached =
            read_cache("proj-ow", Some("overwrite-test"), &key).unwrap();
        assert_eq!(cached.variables.len(), 1);
        assert_eq!(cached.variables[0].value.key, "NEW");

        wipe_cache_for_project("proj-ow").unwrap();
    }

    #[test]
    fn large_variable_set() {
        let (key, _) = test_key();
        use crate::utils::kvpair::KVPair;

        let vars: Vec<DecryptedVariable> = (0..500)
            .map(|i| DecryptedVariable {
                id: format!("var-{}", i),
                value: KVPair::new(
                    format!("KEY_{}", i),
                    format!("value-{}-{}", i, "x".repeat(100)),
                ),
                project_id: "proj-large".into(),
                created_at: format!("2026-01-{:02}T00:00:00Z", (i % 28) + 1),
            })
            .collect();

        write_cache("proj-large", "large-test", &vars, &key).unwrap();
        let cached =
            read_cache("proj-large", Some("large-test"), &key).unwrap();

        assert_eq!(cached.variables.len(), 500);
        for (i, v) in cached.variables.iter().enumerate() {
            assert_eq!(v.value.key, format!("KEY_{}", i));
        }

        wipe_cache_for_project("proj-large").unwrap();
    }

    #[test]
    fn read_by_project_id_prefix_without_name() {
        let (key, _) = test_key();
        let vars = make_vars();

        write_cache("proj-prefix", "some-name", &vars, &key).unwrap();
        let cached = read_cache("proj-prefix", None, &key).unwrap();

        assert_eq!(cached.variables.len(), 2);
        assert_eq!(cached.variables[0].value.key, "DATABASE_URL");

        wipe_cache_for_project("proj-prefix").unwrap();
    }
}
