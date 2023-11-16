use super::*;
use crate::utils::config::get_config;
use anyhow::Ok;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
pub struct NewUserParams {
    pub fingerprint: String,
    pub user_id: String,
    pub pubkey: String,
    pub pubkey_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetEnvParams {
    pub message: String,
    pub allowed_keys: Vec<String>,
    pub project_id: Option<String>,
}

pub fn get_api_url() -> Result<String> {
    Ok(get_config()?
        .sdk_url
        .unwrap_or("http://localhost:3000".into()))
}

#[allow(clippy::upper_case_acronyms)]
pub(crate) struct SDK {}
impl SDK {
    pub async fn new_user(user: &NewUserParams) -> Result<()> {
        let client = reqwest::Client::new();

        let res = client
            .post(&format!("{}/user/new", get_api_url()?))
            .json(&user)
            .send()
            .await?;

        let status = res.status();

        if status.is_success() {
            Ok(())
        } else {
            Err(anyhow!(format!(
                "Failed to create user: {}",
                res.text().await?
            )))
        }
    }

    pub async fn set_env(body: SetEnvParams) -> Result<()> {
        let client = reqwest::Client::new();

        let project_id = match body.project_id {
            Some(pid) => pid,
            None => "null".into(),
        };

        let body = json!({
            "message": body.message,
            "allowed_keys": body.allowed_keys,
            "project_id": project_id
        });

        let res = client
            .post(&format!("{}/secrets/new", get_api_url()?))
            .json(&body)
            .send()
            .await?;

        let status = res.status();

        if status.is_success() {
            Ok(())
        } else {
            Err(anyhow!(format!(
                "Failed to set new secret: {}",
                "err" // res.text().await?
            )))
        }
    }
}
