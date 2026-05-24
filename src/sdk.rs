use super::*;
use crate::{
    types::{ListProjects, ProjectInfo},
    utils::{
        config::Config,
        key::UnlockedKey,
        kvpair::KVPair,
        rpgp::{decrypt_full_many, encrypt},
        variable::{DecryptedVariable, EncryptedVariable, ToKVPair},
    },
};
use anyhow::bail;
use pgp::composed::{Deserializable, SignedPublicKey};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::{header, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;
use utils::variable::DeDupe;

pub fn api_url() -> Url {
    fn try_get_url() -> Result<Url> {
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
    match try_get_url() {
        Ok(u) => u,
        Err(_) => Url::parse("http://localhost:3000")
            .context("Failed to parse URL, this should literally never happen")
            .unwrap(),
    }
}

#[allow(clippy::upper_case_acronyms)]
pub(crate) struct SDK {}
impl SDK {
    // TODO: remove username entirely
    pub async fn new_user(username: &str, public_key: &str) -> Result<String> {
        let client = reqwest::Client::new();

        let body = json!({
            "username": username,
            "public_key": public_key
        });

        let url = api_url().join("/user/new")?;
        let res = client.post(url).json(&body).send().await;

        let res = match res {
            Ok(r) => r.text().await?,
            Err(e) => bail!("Failed to create new user: {}", e.to_string()),
        };

        Ok(res)
    }

    pub async fn get_project_info(
        project_id: &str,
        key: &UnlockedKey,
    ) -> Result<ProjectInfo> {
        // GET /v2/project/:id
        let client = reqwest::Client::new();

        let url = api_url().join("v2/project/")?.join(project_id)?;

        let project_info = client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
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
        project_id: &str,
        key: &UnlockedKey,
    ) -> Result<Vec<String>> {
        let client = reqwest::Client::new();

        let project_info = Self::get_project_info(project_id, key).await?;

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
            .map(|k| encrypt(&k.to_json()?, &pubkeys))
            .collect::<Result<Vec<String>>>()?;

        let body = json!({
            "project_id": project_id,
            "variables": messages,
        });

        #[derive(Serialize, Deserialize, Debug)]
        pub struct SetManyVariableReturnType {
            pub id: String,
        }

        let url = api_url().join("/variables/set-many")?;

        let res = client
            .post(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .json(&body)
            .send()
            .await?;

        let res = res
            .json::<Vec<SetManyVariableReturnType>>()
            .await?
            .iter()
            .map(|r| &r.id)
            .cloned()
            .collect::<Vec<String>>();

        Ok(res)
    }

    pub async fn get_all_variables(
        key: &UnlockedKey,
    ) -> Result<Vec<DecryptedVariable>> {
        let client = reqwest::Client::new();

        let mut url = api_url();
        url.set_path(&format!(
            "/user/{}/variables",
            key.key
                .uuid
                .clone()
                .context("No UUID for key, try `envx upload`")?
        ));

        let response = match client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if let Some(status) = e.status() {
                    // using a match so that we can expand on the error handling later
                    match status {
                        StatusCode::UNAUTHORIZED => {
                            bail!("for some reason, you are unauthorized")
                            // bail!("You do not have access to the project {}. Please ask the owner to add you to the project.", project_id);
                        }
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            bail!("Server error ocurred: {}", e.to_string());
                        }
                        _ => {
                            bail!("Failed to get variables due to unexpected Error Code: {}\n{}", status, e.to_string());
                        }
                    }
                } else {
                    bail!("Failed to get variables: {}", e.to_string());
                }
            }
        };
        let encrypted = response
            .json::<Vec<EncryptedVariable>>()
            .await
            .context("Failed to parse API response into EncryptedVariables")?;

        let decrypted = decrypt_full_many(
            encrypted
                .iter()
                .map(|e| e.value.clone())
                .collect::<Vec<String>>(),
            key,
        )?;

        let kvpairs = decrypted
            .iter()
            .map(|d| KVPair::from_json(d))
            .collect::<Result<Vec<KVPair>>>()?;

        let parsed = encrypted
            .into_iter()
            .zip(kvpairs.into_iter())
            .map(|(d, e)| DecryptedVariable {
                id: d.id,
                value: e,
                project_id: d.project_id,
                created_at: d.created_at,
            })
            .collect::<Vec<DecryptedVariable>>();

        Ok(parsed)
    }

    /// You're probably looking for `get_variables_pruned` instead
    pub async fn get_variables(
        project_id: &str,
        key: &UnlockedKey,
    ) -> Result<Vec<DecryptedVariable>> {
        // url : /project/:id/variables
        let client = reqwest::Client::new();

        let url =
            api_url().join(&format!("/project/{}/variables", project_id))?;

        let response = match client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if let Some(status) = e.status() {
                    // using a match so that we can expand on the error handling later
                    match status {
                        StatusCode::UNAUTHORIZED => {
                            bail!("You do not have access to the project {}. Please ask the owner to add you to the project.", project_id);
                        }
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            bail!("Server error ocurred: {}", e.to_string());
                        }
                        _ => {
                            bail!("Failed to get variables due to unexpected Error Code: {}\n{}", status, e.to_string());
                        }
                    }
                } else {
                    bail!("Failed to get variables: {}", e.to_string());
                }
            }
        };

        let encrypted = response
            .json::<Vec<EncryptedVariable>>()
            .await
            .context("Failed to parse API response into EncryptedVariables")?;

        let decrypted = decrypt_full_many(
            encrypted
                .iter()
                .map(|e| e.value.clone())
                .collect::<Vec<String>>(),
            key,
        )?;

        let kvpairs = decrypted
            .iter()
            .map(|d| KVPair::from_json(d))
            .collect::<Result<Vec<KVPair>>>()?;

        let parsed = encrypted
            .into_iter()
            .zip(kvpairs.into_iter())
            .map(|(d, e)| DecryptedVariable {
                id: d.id,
                value: e,
                project_id: d.project_id,
                created_at: d.created_at,
            })
            .collect::<Vec<DecryptedVariable>>();

        Ok(parsed)
    }

    /// Return variables as a list of kv pairs
    ///
    /// Sorted, and pruned of duplicates (by created_at date)
    #[allow(dead_code)]
    pub async fn get_variables_pruned(
        project_id: &str,
        key: &UnlockedKey,
    ) -> Result<Vec<KVPair>> {
        let variables = Self::get_variables(project_id, key)
            .await
            .context("Failed to get variables")?;

        let kvpairs = variables.dedupe().to_kvpair();
        Ok(kvpairs)
    }

    pub async fn delete_project(
        key: &UnlockedKey,
        project_id: &str,
    ) -> Result<()> {
        // url: /project/:id
        let client = reqwest::Client::new();

        let url = api_url().join(&format!("/project/{}", project_id))?;

        let res = client
            .delete(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await?;

        let status = res.status();

        if status.is_success() {
            Ok(())
        } else {
            bail!("Failed to delete project: {}", res.text().await?)
        }
    }

    pub async fn delete_variable(
        variable_id: &str,
        key: &UnlockedKey,
    ) -> Result<()> {
        // url: DELETE /variables/:id
        let client = reqwest::Client::new();

        let url = api_url().join("variables/")?.join(variable_id)?;

        client
            .delete(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await?;

        Ok(())
    }

    pub async fn list_projects(key: &UnlockedKey) -> Result<Vec<ListProjects>> {
        // GET /v2/projects
        let client = reqwest::Client::new();

        let url = api_url().join("v2/projects")?;

        let res = client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await
            .context("Failed to get projects")?;

        let projects = res
            .json::<Vec<ListProjects>>()
            .await
            .context("Failed to parse API response into Vec<ProjectInfo>")?;

        let project_data = projects
            .iter()
            .map(|p| ListProjects {
                project_id: p.project_id.clone(),
                project_name: p.project_name.clone(),
            })
            .collect::<Vec<ListProjects>>();

        Ok(project_data)
    }

    pub async fn new_project(
        key: &UnlockedKey,
        project_name: &str,
    ) -> Result<String> {
        // POST /v2/projects/new
        let client = reqwest::Client::new();

        let body = json!({
            "name": project_name
        });

        let res = client
            .post(api_url().join("v2/projects/new")?)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        Ok(res)
    }
}
