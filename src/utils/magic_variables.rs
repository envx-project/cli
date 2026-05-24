use crate::{sdk::SDK, utils::variable::ToKVPair};

use super::{
    cache::{read_cache, write_cache},
    key::UnlockedKey,
    kvpair::KVPair,
    variable::DeDupe,
};

pub async fn get_variables_magic(
    project_id: &str,
    key: &UnlockedKey,
    all: bool,
    local: bool,
) -> anyhow::Result<Vec<KVPair>> {
    if local {
        let cached = read_cache(project_id, None, key)?;
        let vars = if all {
            cached.variables.to_kvpair()
        } else {
            cached.variables.dedupe().to_kvpair()
        };
        return Ok(vars);
    }

    match fetch_and_cache(project_id, key, all).await {
        Ok(kvpairs) => Ok(kvpairs),
        Err(fetch_err) => match read_cache(project_id, None, key) {
            Ok(cached) => {
                let age = chrono::Utc::now() - cached.cached_at;
                eprintln!(
                        "warning: using cached variables from {} ago (network fetch failed: {})",
                        humanize_duration(age),
                        fetch_err,
                    );
                let vars = if all {
                    cached.variables.to_kvpair()
                } else {
                    cached.variables.dedupe().to_kvpair()
                };
                Ok(vars)
            }
            Err(_) => Err(fetch_err),
        },
    }
}

async fn fetch_and_cache(
    project_id: &str,
    key: &UnlockedKey,
    all: bool,
) -> anyhow::Result<Vec<KVPair>> {
    let variables = SDK::get_variables(project_id, key).await?;

    let project_name = match SDK::list_projects(key).await {
        Ok(projects) => projects
            .iter()
            .find(|p| p.project_id == project_id)
            .map(|p| p.project_name.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        Err(_) => "unknown".to_string(),
    };

    if let Err(e) = write_cache(project_id, &project_name, &variables, key) {
        eprintln!("warning: failed to write variable cache: {}", e);
    }

    let kvpairs = if all {
        variables.to_kvpair()
    } else {
        variables.dedupe().to_kvpair()
    };

    Ok(kvpairs)
}

fn humanize_duration(d: chrono::Duration) -> String {
    let total_secs = d.num_seconds();
    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        format!("{}m", total_secs / 60)
    } else if total_secs < 86400 {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, mins)
        }
    } else {
        let days = total_secs / 86400;
        format!("{}d", days)
    }
}
