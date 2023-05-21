use std::fmt::Display;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{commands::BASE_URL, utils::config::get_config};

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

pub(crate) async fn get_variables() -> Option<Vec<Env>> {
    let config = get_config().unwrap();
    let client = Client::new();

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

    Some(res)
}
