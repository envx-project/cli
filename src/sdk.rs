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

use pgp::composed::{Deserializable, SignedPublicKey};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;
use utils::variable::DeDupe;

pub fn api_url() -> Result<Url> {
    Config::load()?.sdk_url()
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

        let url = api_url()?.join("/user/new")?;
        let response = client
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .context("Failed to create new user")?;
        parse_created_id(&response.text().await?)
    }

    pub async fn get_project_info(
        project_id: &str,
        key: &UnlockedKey,
    ) -> Result<ProjectInfo> {
        // GET /v2/project/:id
        let client = reqwest::Client::new();

        let url = api_url()?.join("v2/project/")?.join(project_id)?;

        let project_info = client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await
            .context("Failed to get project info")?
            .error_for_status()?
            .json::<ProjectInfo>()
            .await
            .context("Failed to parse project info")?;

        Ok(project_info)
    }

    pub async fn replace_many(
        kvpairs: Vec<KVPair>,
        project_id: &str,
        key: &UnlockedKey,
        replace_ids: Vec<String>,
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
            "replace_ids": replace_ids,
        });

        #[derive(Serialize, Deserialize, Debug)]
        pub struct SetManyVariableReturnType {
            pub id: String,
        }

        let endpoint = if replace_ids.is_empty() {
            "/variables/set-many"
        } else {
            "/variables/replace-many"
        };
        let url = api_url()?.join(endpoint)?;

        let res = client
            .post(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .json(&body)
            .send()
            .await?;

        let res = res
            .error_for_status()
            .context("Variable write failed; existing values were not deleted by this client")?
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

        let mut url = api_url()?;
        url.set_path(&format!(
            "/user/{}/variables",
            key.key
                .uuid
                .clone()
                .context("No UUID for key, try `envx upload`")?
        ));

        let response = client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await
            .context("Failed to get variables")?
            .error_for_status()
            .context("Server rejected variable request")?;
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
            api_url()?.join(&format!("/project/{}/variables", project_id))?;

        let response = client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await
            .context("Failed to get variables")?
            .error_for_status()
            .context("Server rejected variable request")?;
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

        let url = api_url()?.join(&format!("/project/{}", project_id))?;

        client
            .delete(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await?
            .error_for_status()
            .context("Failed to delete project")?;
        Ok(())
    }

    pub async fn delete_variable(
        variable_id: &str,
        key: &UnlockedKey,
    ) -> Result<()> {
        // url: DELETE /variables/:id
        let client = reqwest::Client::new();

        let url = api_url()?.join("variables/")?.join(variable_id)?;

        client
            .delete(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn list_projects(key: &UnlockedKey) -> Result<Vec<ListProjects>> {
        // GET /v2/projects
        let client = reqwest::Client::new();

        let url = api_url()?.join("v2/projects")?;

        let res = client
            .get(url)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .send()
            .await
            .context("Failed to get projects")?
            .error_for_status()?;

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
            .post(api_url()?.join("v2/projects/new")?)
            .header(header::AUTHORIZATION, key.auth_token()?.bearer())
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        parse_created_id(&res)
    }
}

fn parse_created_id(body: &str) -> Result<String> {
    Ok(uuid::Uuid::parse_str(body.trim())
        .context("Server returned an invalid resource UUID")?
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_error_bodies_before_they_can_be_saved_as_ids() {
        assert!(parse_created_id("Internal Server Error").is_err());
        assert!(parse_created_id(r#"{"error":"unauthorized"}"#).is_err());
        assert!(parse_created_id("").is_err());
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(parse_created_id(id).unwrap(), id);
    }
}
