use anyhow::Result;
use std::{collections::HashMap, fmt};

use crate::sdk::SDK;

use super::{
    config::{Config, Project},
    key::UnlockedKey,
};

#[derive(Debug)]
struct DisplayProject<'a> {
    project_id: &'a str,
    project_name: &'a str,
    path: &'a str,
}

impl fmt::Display for DisplayProject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {} - {}",
            self.project_id, self.project_name, self.path
        )
    }
}

pub struct Choice {}
impl Choice {
    pub async fn choose_project(
        projects: &[Project],
        key: &UnlockedKey,
    ) -> Result<String> {
        let all_projects = SDK::list_projects(key).await?;

        let project_name_map: HashMap<_, _> = all_projects
            .iter()
            .map(|p| (&p.project_id, &p.project_name))
            .collect();

        let mut options = projects
            .iter()
            .map(|p| {
                let pname = project_name_map.get(&p.project_id);
                DisplayProject {
                    project_id: &p.project_id,
                    project_name: match pname {
                        Some(n) => n,
                        None => "<unnamed>",
                    },
                    path: p.path.to_str().unwrap(),
                }
            })
            .collect::<Vec<_>>();

        all_projects.iter().for_each(|p| {
            let project_name = match p.project_name.trim().is_empty() {
                true => "<unnamed>",
                false => &p.project_name,
            };
            options.push(DisplayProject {
                project_id: &p.project_id,
                project_name,
                path: "Remote",
            });
        });

        if options.is_empty() {
            return Err(anyhow::anyhow!("No projects found"));
        }

        crate::utils::prompt::require_interactive(
            "No project selected and stdin is not a terminal.",
            "Pass --project-id <id> or run `envx link` to link one to this directory.",
        )?;

        let selected =
            crate::utils::prompt::prompt_options("Select project", options)?;

        Ok(selected.project_id.to_string())
    }

    pub async fn try_project(
        project_id: Option<String>,
        key: &UnlockedKey,
    ) -> Result<String> {
        match project_id {
            Some(p) => Ok(p),
            None => {
                let config = Config::get();
                let project = config.get_project();

                match project {
                    Ok(p) => Ok(p.project_id.clone()),
                    Err(_) => Self::choose_project(&config.projects, key).await,
                }
            }
        }
    }
}
