use super::*;
use crate::{
    types::ProjectInfo,
    utils::{
        auth::get_token,
        config::get_config,
        kvpair::KVPair,
        partial_variable::{ParsedPartialVariable, PartialVariable, ToKVPair, ToParsed},
        rpgp::{decrypt_full_many, encrypt_multi},
    },
};
use anyhow::bail;
use pgp::{Deserializable, SignedPublicKey};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct SetEnvParams {
    pub message: String,
    pub allowed_keys: Vec<String>,
    pub project_id: Option<String>,
}

pub fn get_api_url() -> Url {
    fn try_get_url() -> Result<Url> {
        let url = get_config()?.sdk_url.ok_or(anyhow!("No SDK URL set"))?;
        let url = Url::parse(&url)?;
        Ok(url)
    }
    match try_get_url() {
        Ok(u) => u,
        Err(_) => Url::parse("http://localhost:3000").unwrap(),
    }
}

#[allow(clippy::upper_case_acronyms)]
pub(crate) struct SDK {}
impl SDK {
    async fn auth_header(partial_fingerprint: &str) -> Result<String> {
        let config = get_config()?;
        let key = config.get_key(partial_fingerprint)?;
        let Some(uuid) = key.uuid.clone() else {
            bail!("No UUID for key {}\nTry envx upload", partial_fingerprint)
        };
        let auth_token = get_token(&key.fingerprint, &uuid).await?;
        Ok(format!("Bearer {}", auth_token))
    }

    pub async fn new_user(username: &str, public_key: &str) -> Result<String> {
        let client = reqwest::Client::new();

        let body = json!({
            "username": username,
            "public_key": public_key
        });

        let url = get_api_url().join("/user/new")?;
        let res = client.post(url).json(&body).send().await;

        let res = match res {
            Ok(r) => r.text().await?,
            Err(e) => {
                dbg!(&e);

                return Err(anyhow!(format!(
                    "Failed to create new user: {}",
                    e.to_string()
                )));
            }
        };

        Ok(res)
    }

    pub async fn get_project_info(
        project_id: &str,
        partial_fingerprint: &str,
    ) -> Result<ProjectInfo> {
        let client = reqwest::Client::new();

        let url = get_api_url().join("project/")?.join(project_id)?;

        let project_info = client
            .get(url)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await
            .context("Failed to get project info")?
            .json::<ProjectInfo>()
            .await
            .context("Failed to parse project info")?;

        Ok(project_info)
    }

    pub async fn set_many(
        kvpairs: Vec<KVPair>,
        partial_fingerprint: &str,
        project_id: &str,
    ) -> Result<Vec<String>> {
        let client = reqwest::Client::new();

        let project_info = Self::get_project_info(project_id, partial_fingerprint).await?;

        let recipients = project_info
            .users
            .iter()
            .map(|u| u.public_key.as_str())
            .collect::<Vec<&str>>();

        let pubkeys = recipients
            .iter()
            .map(|k| Ok(SignedPublicKey::from_string(k)?.0))
            .collect::<Result<Vec<SignedPublicKey>>>()?;

        let messages = kvpairs
            .par_iter()
            .map(|k| encrypt_multi(&k.to_json().unwrap(), &pubkeys).unwrap())
            .collect::<Vec<String>>();

        let body = json!({
            "project_id": project_id,
            "variables": messages,
        });

        #[derive(Serialize, Deserialize, Debug)]
        pub struct SetManyVariableReturnType {
            pub id: String,
        }

        let url = get_api_url().join("/variables/set-many")?;

        let res = client
            .post(url)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .json(&body)
            .send()
            .await?;

        let res = res.json::<Vec<SetManyVariableReturnType>>().await?;

        Ok(res.iter().map(|r| r.id.clone()).collect())
    }

    pub async fn get_all_variables(
        partial_fingerprint: &str,
    ) -> Result<(Vec<KVPair>, Vec<ParsedPartialVariable>)> {
        // GET /user/:id/variables
        let config = get_config()?;
        let key = config.get_key(partial_fingerprint)?;

        let client = reqwest::Client::new();

        let mut url = get_api_url();
        url.set_path(&format!("/user/{}/variables", key.uuid.unwrap()));

        let encrypted = client
            .get(url)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await
            .context("Failed to get variables")?
            .json::<Vec<PartialVariable>>()
            .await
            .context("Failed to parse API response into PartialVariables")?;

        let decrypted = decrypt_full_many(
            encrypted
                .clone()
                .iter()
                .map(|e| e.value.clone())
                .collect::<Vec<String>>(),
            &get_config().unwrap(),
        )?;

        let partials = decrypted
            .iter()
            .zip(encrypted.iter())
            .map(|(d, e)| ParsedPartialVariable {
                id: e.id.clone(),
                value: KVPair::from_json(d.clone()).unwrap(),
                project_id: e.project_id.clone(),
                created_at: e.created_at.clone(),
            })
            .collect::<Vec<ParsedPartialVariable>>();

        let parsed = decrypted
            .iter()
            .map(|d| KVPair::from_json(d.clone()).unwrap())
            .collect::<Vec<KVPair>>();

        Ok((parsed, partials))
    }

    /// You're probably looking for `get_variables_pruned` instead
    pub async fn get_variables(
        project_id: &str,
        partial_fingerprint: &str,
    ) -> Result<(Vec<KVPair>, Vec<PartialVariable>)> {
        // url : /project/:id/variables
        let client = reqwest::Client::new();

        let url = get_api_url().join(&format!("/project/{}/variables", project_id))?;

        let encrypted = client
            .get(url)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await
            .context("Failed to get variables")?
            .json::<Vec<PartialVariable>>()
            .await
            .context("Failed to parse API response into PartialVariables")?;

        let decrypted = decrypt_full_many(
            encrypted
                .clone()
                .iter()
                .map(|e| e.value.clone())
                .collect::<Vec<String>>(),
            &get_config().unwrap(),
        )?;

        // splice decrypted and encrypted into a Vector of PartialKey
        let partials = decrypted
            .iter()
            .zip(encrypted.iter())
            .map(|(d, e)| PartialVariable {
                id: e.id.clone(),
                value: d.clone(),
                project_id: e.project_id.clone(),
                created_at: e.created_at.clone(),
            })
            .collect::<Vec<PartialVariable>>();

        let parsed = decrypted
            .iter()
            .map(|d| KVPair::from_json(d.clone()).unwrap())
            .collect::<Vec<KVPair>>();

        Ok((parsed, partials))
    }

    /// Return variables as a list of kv pairs
    ///
    /// Sorted, and pruned of duplicates (by created_at date)
    pub async fn get_variables_pruned(
        project_id: &str,
        partial_fingerprint: &str,
    ) -> Result<Vec<KVPair>> {
        let (kvpairs, partial) = Self::get_variables(project_id, partial_fingerprint)
            .await
            .context("Failed to get variables")?;
        let mut pruned = partial.zip_to_parsed(kvpairs).to_kvpair();
        pruned.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(pruned)
    }

    pub async fn get_user(
        partial_fingerprint: &str,
        user_to_get: &str,
    ) -> Result<(String, String)> {
        // url: /user/:id
        let client = reqwest::Client::new();

        #[derive(Serialize, Deserialize, Debug)]
        pub struct StrippedUser {
            pub id: String,
            pub public_key: String,
        }

        let url = get_api_url().join("user/")?.join(user_to_get)?;

        let user = client
            .get(url)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await?
            .json::<StrippedUser>()
            .await?;

        Ok((user.id, user.public_key))
    }

    pub async fn add_user_to_project(
        partial_fingerprint: &str,
        user_to_add: &str,
        project_id: &str,
    ) -> Result<()> {
        // url: /project/:id/add-user
        let client = reqwest::Client::new();

        let body = json!({
            "user_id": user_to_add
        });

        let url = get_api_url().join(&format!("/project/{}/add-user", project_id))?;

        let res = client
            .post(url.join(&format!("/project/{}/add-user", project_id))?)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .json(&body)
            .send()
            .await?;

        let status = res.status();

        if status.is_success() {
            Ok(())
        } else {
            Err(anyhow!(format!(
                "Failed to add user to project: {}",
                res.text().await?
            )))
        }
    }

    pub async fn delete_project(partial_fingerprint: &str, project_id: &str) -> Result<()> {
        // url: /project/:id
        let client = reqwest::Client::new();

        let url = get_api_url().join(&format!("/project/{}", project_id))?;

        let res = client
            .delete(url)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await?;

        let status = res.status();

        if status.is_success() {
            Ok(())
        } else {
            Err(anyhow!(format!(
                "Failed to delete project: {}",
                res.text().await?
            )))
        }
    }

    pub async fn delete_variable(variable_id: &str, partial_fingerprint: &str) -> Result<()> {
        // url: DELETE /variables/:id
        let client = reqwest::Client::new();

        let url = get_api_url().join("variables/")?.join(variable_id)?;

        client
            .delete(url)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await?;

        Ok(())
    }

    pub async fn list_projects(partial_fingerprint: &str) -> Result<Vec<String>> {
        // GET /projects
        let client = reqwest::Client::new();

        let res = client
            .get(get_api_url().join("projects")?)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await
            .context("Failed to get projects")?;

        let res = res
            .json::<Vec<String>>()
            .await
            .context("Failed to parse API response into Vec<String>")?;

        Ok(res)
    }

    pub async fn new_project(partial_fingerprint: &str) -> Result<String> {
        // POST /projects/new
        let client = reqwest::Client::new();

        let res = client
            .post(get_api_url().join("projects/new")?)
            .header(
                header::AUTHORIZATION,
                Self::auth_header(partial_fingerprint).await?,
            )
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
    }
}
