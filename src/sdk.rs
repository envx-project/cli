use anyhow::Ok;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::*;

#[derive(Serialize, Deserialize)]
pub struct NewUserParams {
    pub fingerprint: String,
    pub user_id: String,
    pub pubkey: String,
    pub pubkey_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct SetEnvParams {
    pub message: String,
    pub allowed_keys: Vec<String>,
    pub project_id: Option<String>,
}

const API_URL: &str = "http://localhost:3000";

pub(crate) struct SDK {}
impl SDK {
    pub async fn new_user(user: &NewUserParams) -> Result<()> {
        let client = reqwest::Client::new();

        let res = client
            .post(&format!("{}/user/new", API_URL))
            .json(&user)
            .send()
            .await?;

        let status = res.status();

        if status.is_success() {
            Ok(())
        } else {
            Err(anyhow::Error::msg(format!(
                "Failed to create user: {}",
                res.text().await?
            )))
        }
    }

    pub async fn set_env(body: SetEnvParams) -> Result<()> {
        let client = reqwest::Client::new();

        let res = client
            .post(&format!("{}/secrets/new", API_URL))
            .json(&body)
            .send()
            .await?;

        let status = res.status();

        dbg!(status);

        if status.is_success() {
            Ok(())
        } else {
            Err(anyhow::Error::msg(format!(
                "Failed to set new secret: {}",
                res.text().await?
            )))
        }
    }
}
