use anyhow::Ok;
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Serialize, Deserialize)]
pub struct NewUserParams {
    pub fingerprint: String,
    pub user_id: String,
    pub pubkey: String,
    pub pubkey_hash: String,
}

pub(crate) struct Client {
    pub(crate) api_url: String,
}
impl Client {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            api_url: "http://localhost:3000".to_string(),
        })
    }

    pub async fn new_user(&self, user: &NewUserParams) -> Result<()> {
        let client = reqwest::Client::new();

        let res = client
            .post(&format!("{}/users", self.api_url))
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
}
