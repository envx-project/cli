use crate::utils::{config::Config, key::Key, rpgp::get_vault_location};
use anyhow::{bail, Context, Result};
use pgp::{
    composed::{
        ArmorOptions, Deserializable, MessageBuilder, SignedPublicKey,
        SignedSecretKey,
    },
    crypto::hash::HashAlgorithm,
    types::KeyDetails,
};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    version: u8,
    user_id: String,
    fingerprint: String,
    secret_key: String,
}
impl Bundle {
    pub fn from_key(key: &Key) -> Result<Self> {
        let bundle = Self {
            version: 1,
            user_id: key.uuid.clone().context("Identity is not registered")?,
            fingerprint: key.fingerprint.clone(),
            secret_key: key.secret_key_str()?,
        };
        bundle.parse()?;
        Ok(bundle)
    }
    fn parse(&self) -> Result<SignedSecretKey> {
        if self.version != 1 || self.secret_key.len() > 60000 {
            bail!("Unsupported identity bundle");
        }
        uuid::Uuid::parse_str(&self.user_id).context("Invalid account ID")?;
        let (secret, _) = SignedSecretKey::from_string(&self.secret_key)
            .context("Invalid transferred identity")?;
        secret.verify().context("Invalid identity self-signature")?;
        if hex::encode(secret.fingerprint().as_bytes()) != self.fingerprint {
            bail!("Identity fingerprint mismatch");
        }
        if !secret.primary_key.secret_params().is_encrypted()
            || secret
                .secret_subkeys
                .iter()
                .any(|s| !s.key.secret_params().is_encrypted())
        {
            bail!("Only passphrase-protected identities can be transferred");
        }
        Ok(secret)
    }
    pub fn validate(&self, password: &str) -> Result<(Key, String, String)> {
        crate::utils::key::validate_passphrase_not_empty(password)?;
        let secret = self.parse()?;
        secret
            .primary_key
            .unlock(&password.into(), |_, _| Ok(()))?
            .context("Passphrase does not unlock transferred key")?;
        for sub in &secret.secret_subkeys {
            sub.key
                .unlock(&password.into(), |_, _| Ok(()))?
                .context("Passphrase does not unlock transferred subkey")?;
        }
        let public = SignedPublicKey::from(secret.clone());
        let public_armor = public.to_armored_string(ArmorOptions::default())?;
        let mut builder =
            MessageBuilder::from_bytes("", chrono::Utc::now().to_string());
        builder.sign(
            &secret.primary_key,
            password.into(),
            HashAlgorithm::Sha3_512,
        );
        let signature = builder
            .to_armored_string(rand::rngs::OsRng, ArmorOptions::default())?;
        let token = crate::utils::auth_token::AuthToken::new(
            self.user_id.clone(),
            signature,
        )
        .to_string();
        let user = public
            .details
            .users
            .first()
            .context("Identity has no user ID")?;
        let key = Key {
            fingerprint: self.fingerprint.clone(),
            uuid: Some(self.user_id.clone()),
            note: "Primary Key".into(),
            primary_user_id: String::from_utf8_lossy(user.id.id()).into_owned(),
            pubkey_only: Some(false),
        };
        Ok((key, public_armor, token))
    }
    pub fn install(
        &self,
        config: &mut Config,
        server: &str,
        key: Key,
        public: &str,
    ) -> Result<()> {
        let vault = get_vault_location()?;
        fs::create_dir_all(&vault)?;
        let destination = vault.join(&key.fingerprint);
        if destination.exists() {
            // Recover a prior publication whose final settings write failed.
            if fs::read_to_string(destination.join("private.key"))?
                == self.secret_key
                && fs::read_to_string(destination.join("public.key"))? == public
            {
                return config.install_identity(key, server);
            }
            bail!("Different key files already exist; login will not overwrite them");
        }
        let temporary =
            vault.join(format!(".pairing-{}", uuid::Uuid::new_v4()));
        let mut directory = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            directory.mode(0o700);
        }
        directory.create(&temporary)?;
        let result = (|| -> Result<()> {
            for (name, data) in [
                ("private.key", self.secret_key.as_str()),
                ("public.key", public),
            ] {
                let mut options = fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let mut file = options.open(temporary.join(name))?;
                file.write_all(data.as_bytes())?;
                file.sync_all()?;
            }
            fs::rename(&temporary, &destination)?;
            // Publish settings last. A failed settings write leaves recoverable encrypted files.
            config.install_identity(key, server)?;
            Ok(())
        })();
        if temporary.exists() {
            let _ = fs::remove_dir_all(&temporary);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::Bundle;
    use pgp::{
        composed::{ArmorOptions, KeyType, SecretKeyParamsBuilder},
        types::KeyDetails,
    };
    #[test]
    fn bundle_requires_matching_protected_key_and_correct_password() {
        let secret = SecretKeyParamsBuilder::default()
            .key_type(KeyType::Ed25519)
            .can_sign(true)
            .primary_user_id("synthetic".into())
            .passphrase(Some("fixture-passphrase".into()))
            .build()
            .unwrap()
            .generate(rand::rngs::OsRng)
            .unwrap()
            .sign(rand::rngs::OsRng, &"fixture-passphrase".into())
            .unwrap();
        let mut bundle = Bundle {
            version: 1,
            user_id: uuid::Uuid::new_v4().to_string(),
            fingerprint: hex::encode(secret.fingerprint().as_bytes()),
            secret_key: secret
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        };
        assert!(bundle.validate("fixture-passphrase").is_ok());
        assert!(bundle.validate("wrong-password").is_err());
        bundle.fingerprint = "../../escape".into();
        assert!(bundle.validate("fixture-passphrase").is_err());
    }
    use crate::utils::{config::Config, key::Key};
    #[test]
    fn interrupted_install_can_resume_only_with_identical_key_files() {
        let mut config = Config::load().unwrap();
        let key = Key {
            fingerprint: "c".repeat(40),
            note: "test".into(),
            primary_user_id: "test".into(),
            pubkey_only: Some(false),
            uuid: Some(uuid::Uuid::new_v4().to_string()),
        };
        let directory = crate::utils::rpgp::get_vault_location()
            .unwrap()
            .join(&key.fingerprint);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("private.key"), "protected-fixture")
            .unwrap();
        std::fs::write(directory.join("public.key"), "public-fixture").unwrap();
        let mut bundle = Bundle {
            version: 1,
            user_id: key.uuid.clone().unwrap(),
            fingerprint: key.fingerprint.clone(),
            secret_key: "different-fixture".into(),
        };
        assert!(bundle
            .install(
                &mut config,
                "https://api.envx.sh/",
                key.clone(),
                "public-fixture"
            )
            .is_err());
        assert!(Config::load().unwrap().primary_key.is_none());
        bundle.secret_key = "protected-fixture".into();
        bundle
            .install(
                &mut config,
                "https://api.envx.sh/",
                key.clone(),
                "public-fixture",
            )
            .unwrap();
        assert_eq!(
            Config::load().unwrap().primary_key.unwrap().fingerprint,
            key.fingerprint
        );
        assert_eq!(
            std::fs::read_to_string(directory.join("private.key")).unwrap(),
            "protected-fixture"
        );
    }
    #[test]
    fn concurrent_identity_install_never_replaces_an_existing_account() {
        let mut first = Config::load().unwrap();
        let mut stale = Config::load().unwrap();
        let key = Key {
            fingerprint: "a".repeat(40),
            note: "test".into(),
            primary_user_id: "test".into(),
            pubkey_only: Some(false),
            uuid: Some(uuid::Uuid::new_v4().to_string()),
        };
        first
            .install_identity(key.clone(), "https://api.envx.sh/")
            .unwrap();
        let mut other = key.clone();
        other.fingerprint = "b".repeat(40);
        assert!(stale
            .install_identity(other, "https://api.envx.sh/")
            .is_err());
        assert_eq!(
            Config::load().unwrap().primary_key.unwrap().fingerprint,
            key.fingerprint
        );
        assert!(stale.primary_key.is_none());
    }
}
