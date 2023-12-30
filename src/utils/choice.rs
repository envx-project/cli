use anyhow::{Context, Ok, Result};

use crate::sdk::SDK;

use super::{
    config::{get_config, Config},
    key::Key,
};

pub struct Choice {}
impl Choice {
    pub fn get_key(partial_fingerprint: &str) -> Result<(Key, Config)> {
        let config = get_config().context("Failed to get config")?;
        let key = config.get_key(partial_fingerprint)?;
        Ok((key, config))
    }

    pub async fn choose_project(partial_fingerprint: &str) -> Result<String> {
        let (key, config) = Self::get_key(partial_fingerprint)?;

        let all_projects = SDK::list_projects(&key.fingerprint).await?;
        let local_projects = config.projects.clone();

        let all_projects = all_projects
            .iter()
            .filter(|p| !local_projects.iter().any(|lp| &lp.project_id == *p))
            .collect::<Vec<_>>();

        let mut options = local_projects
            .iter()
            .map(|p| format!("{} - {}", p.project_id, p.path.to_str().unwrap()))
            .collect::<Vec<_>>();

        all_projects.iter().for_each(|p| {
            options.push(format!("{} - {}", p, "Remote"));
        });

        let selected = crate::utils::prompt::prompt_options("Select project", options)?;

        if selected.is_empty() {
            return Err(anyhow::anyhow!("No project selected"));
        }

        let selected = selected
            .split(" - ")
            .map(|s| s.to_string())
            .collect::<Vec<String>>()[0]
            .clone();

        Ok(selected)
    }
}
