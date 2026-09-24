//! Versioned atomic project rewrap protocol, kept separate from generated SDK code.
use super::{
    config::Config,
    key::UnlockedKey,
    rpgp,
    variable::{DecryptedVariable, EncryptedVariable},
};
use anyhow::{bail, Context, Result};
use pgp::composed::{Deserializable, SignedPublicKey};
use rayon::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const VERSION: u8 = 1;
#[derive(Deserialize)]
pub struct Recipient {
    pub id: String,
    pub public_key: String,
}
#[derive(Deserialize)]
pub struct Snapshot {
    pub protocol_version: u8,
    pub project_id: String,
    pub snapshot: String,
    pub variables: Vec<EncryptedVariable>,
    pub users: Vec<Recipient>,
}
#[derive(Serialize, Deserialize)]
pub struct InvitePayload {
    pub protocol_version: u8,
    pub project_id: String,
    pub snapshot: String,
    pub variables: Vec<DecryptedVariable>,
}
#[derive(Deserialize)]
pub struct PreparedInvite {
    pub protocol_version: u8,
    pub project_id: String,
    pub invite_id: String,
    pub ciphertext: String,
    pub source_snapshot: String,
    pub snapshot: String,
    pub users: Vec<Recipient>,
}
#[derive(Serialize)]
pub struct RewrappedVariable {
    pub id: String,
    pub value: String,
}

pub struct Client {
    client: reqwest::Client,
    base: String,
    token: String,
}
impl Client {
    pub fn new(config: &Config, key: &UnlockedKey) -> Result<Self> {
        let sdk = config.sdk_configuration(key)?;
        Ok(Self {
            client: reqwest::Client::new(),
            base: sdk.base_path,
            token: sdk
                .bearer_access_token
                .context("Missing authentication token")?,
        })
    }
    pub async fn post(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<reqwest::Response> {
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            // Never include raw server bodies: a failure may echo encrypted payloads.
            let error = response.json::<Value>().await.ok();
            let code = error
                .as_ref()
                .and_then(|value| value.get("code"))
                .and_then(Value::as_str);
            match code {
                Some("project_snapshot_stale") => bail!("Project or recipients changed. Regenerate the invitation or retry add-users from a fresh snapshot."),
                Some("invite_upgrade_required") => bail!("Upgrade envx and regenerate the invitation."),
                Some("invite_already_accepted") => bail!("Invitation already accepted."),
                _ => bail!("Project sharing request failed ({status}). No successful sharing operation was confirmed."),
            }
        }
        Ok(response)
    }
    pub async fn json<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T> {
        Ok(self.post(path, body).await?.json().await?)
    }
    pub async fn snapshot(
        &self,
        project: &str,
        added: &[uuid::Uuid],
    ) -> Result<Snapshot> {
        let value: Snapshot = self
            .json(
                &format!("/v2/project/{project}/snapshot"),
                &serde_json::json!({"add_user_ids":added}),
            )
            .await?;
        if value.protocol_version != VERSION || value.project_id != project {
            bail!(
                "Unsupported or inconsistent project snapshot; upgrade envx."
            );
        }
        Ok(value)
    }
}
pub fn decrypt(
    snapshot: &Snapshot,
    key: &UnlockedKey,
) -> Result<Vec<DecryptedVariable>> {
    let values = rpgp::decrypt_full_many(
        snapshot.variables.iter().map(|v| v.value.clone()).collect(),
        key,
    )?;
    values
        .into_iter()
        .zip(&snapshot.variables)
        .map(|(value, encrypted)| {
            Ok(DecryptedVariable {
                id: encrypted.id.clone(),
                project_id: encrypted.project_id.clone(),
                created_at: encrypted.created_at.clone(),
                value: serde_json::from_str(&value)?,
            })
        })
        .collect()
}
pub fn rewrap(
    variables: &[DecryptedVariable],
    project: &str,
    recipients: &[Recipient],
    key: &UnlockedKey,
) -> Result<Vec<RewrappedVariable>> {
    let own_id = key
        .key
        .uuid
        .as_deref()
        .context("Key has not been uploaded")?;
    let own_public = key.key.public_key_str()?;
    if !recipients
        .iter()
        .any(|r| r.id == own_id && r.public_key == own_public)
    {
        bail!("Recipient snapshot does not contain your current public key.");
    }
    let mut seen = HashSet::new();
    for variable in variables {
        if variable.project_id != project || !seen.insert(&variable.id) {
            bail!("Invitation contains inconsistent variable identities; regenerate it.");
        }
    }
    let pubkeys = recipients
        .iter()
        .map(|r| Ok(SignedPublicKey::from_string(&r.public_key)?.0))
        .collect::<Result<Vec<_>>>()?;
    variables
        .par_iter()
        .map(|variable| {
            Ok(RewrappedVariable {
                id: variable.id.clone(),
                value: rpgp::encrypt(&variable.value.to_json()?, &pubkeys)?,
            })
        })
        .collect()
}
