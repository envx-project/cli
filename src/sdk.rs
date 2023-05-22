use crate::{types::User, utils::config::get_config};
use anyhow::Ok;
use serde::{Deserialize, Serialize};
use url::Url;

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

        let base = Url::parse(BASE_URL)?;
        let url = base.join(&format!("/env/{}/{}", config.user_id, key))?;

        let res = client.post(url).json::<SetParams>(&params).send().await?;

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
        let config = get_config()?;
        let client = reqwest::Client::new();

        let base = Url::parse(BASE_URL)?;
        let url = base.join(&format!("/env/{}/{}", config.user_id, key))?;

        let env_var = client
            .get(url)
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

        let base = Url::parse(BASE_URL)?;
        let url = base.join(&format!("/env/{}/{}", config.user_id, key))?;

        let res = client.delete(url).send().await?;

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
        let config = get_config()?;
        let client = reqwest::Client::new();

        let params = GetAllVariablesParams {
            password: config.password.clone(),
        };

        let base = Url::parse(BASE_URL)?;
        let url = base.join(&format!("/env/{}", config.user_id))?;

        let res = client
            .get(url)
            .json(&params)
            .send()
            .await?
            .json::<Vec<Env>>()
            .await?;

        Ok(res)
    }

    pub async fn create_user() -> Result<User, anyhow::Error> {
        let client = reqwest::Client::new();

        let base = Url::parse(BASE_URL)?;
        let url = base.join("/user")?;

        let user = client.post(url).send().await?.json::<User>().await?;

        Ok(user)
    }
}
