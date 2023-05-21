use anyhow::Ok;
use serde::{Deserialize, Serialize};

use crate::utils::config::get_config;

use super::*;
use std::fmt::Display;

#[derive(Serialize, Deserialize)]
struct SetParams {
    password: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetAllVariablesParams {
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Env {
    pub name: String,
    pub value: String,
}

impl Display for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

#[derive(Serialize, Deserialize)]
struct GetParams {
    password: String,
}

pub(crate) struct Client {}
impl Client {
    pub async fn set_env(key: String, value: String) -> Result<(), anyhow::Error> {
        let config = get_config()?;
        let client = reqwest::Client::new();

        let params = SetParams {
            password: config.password.clone(),
            value: value.clone(),
        };

        let res = client
            .post(format!("{}/env/{}/{}", BASE_URL, config.user_id, key))
            .json::<SetParams>(&params)
            .send()
            .await?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::Error::msg(format!(
                "Could not set variable {}={}",
                key, value,
            )))
        }
    }

    #[allow(dead_code)]
    pub async fn get_env(key: String) -> Result<Env, anyhow::Error> {
        // {{base_url}}/env/{{userId}}/test
        let config = get_config()?;
        let client = reqwest::Client::new();

        let env_var = client
            .get(format!("{}/env/{}/{}", BASE_URL, config.user_id, key))
            .json(&GetParams {
                password: config.password.clone(),
            })
            .send()
            .await?
            .json::<Env>()
            .await?;

        Ok(env_var)
    }

    pub async fn delete_env(key: String) -> Result<(), anyhow::Error> {
        let config = get_config()?;
        let client = reqwest::Client::new();

        let res = client
            .delete(format!("{}/env/{}/{}", BASE_URL, config.user_id, key))
            .send()
            .await?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::Error::msg(format!(
                "Could not unset variable {}",
                key.red()
            )))
        }
    }

    pub async fn get_variables() -> Result<Vec<Env>, anyhow::Error> {
        let config = get_config().unwrap();
        let client = reqwest::Client::new();

        let params = GetAllVariablesParams {
            password: config.password.clone(),
        };

        let res = client
            .get(format!("{}/env/{}", BASE_URL, config.user_id.clone()))
            .json(&params)
            .send()
            .await
            .unwrap()
            .json::<Vec<Env>>()
            .await
            .unwrap();

        Ok(res)
    }
}
