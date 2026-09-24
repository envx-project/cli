use crate::{sdk::SDK, utils::variable::ToKVPair};

use super::{
    cache::{read_cache, write_cache},
    key::UnlockedKey,
    kvpair::KVPair,
    variable::DeDupe,
};

pub async fn get_variables_magic(
    config: &super::config::Config,
    project_id: &str,
    key: &UnlockedKey,
    all: bool,
    local: bool,
) -> anyhow::Result<Vec<KVPair>> {
    if local {
        let cached = read_cache(config, project_id, None, key)?;
        let vars = if all {
            cached.variables.to_kvpair()
        } else {
            cached.variables.dedupe().to_kvpair()
        };
        return Ok(vars);
    }

    match fetch_and_cache(config, project_id, key, all).await {
        Ok(kvpairs) => Ok(kvpairs),
        Err(fetch_err) if !allows_cache_fallback(&fetch_err) => Err(fetch_err),
        Err(fetch_err) => match read_cache(config, project_id, None, key) {
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
    config: &super::config::Config,
    project_id: &str,
    key: &UnlockedKey,
    all: bool,
) -> anyhow::Result<Vec<KVPair>> {
    let variables = SDK::get_variables(config, project_id, key).await?;

    let project_name = match SDK::list_projects(config, key).await {
        Ok(projects) => projects
            .iter()
            .find(|p| p.project_id == project_id)
            .map(|p| p.project_name.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        Err(_) => "unknown".to_string(),
    };

    if let Err(e) =
        write_cache(config, project_id, &project_name, &variables, key)
    {
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

// Offline fallback must not override a server's access decision or hide corrupt ciphertext.
fn allows_cache_fallback(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<reqwest::Error>())
        .any(|error| {
            error.is_connect()
                || error.is_timeout()
                || error
                    .status()
                    .is_some_and(|status| status.is_server_error())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[tokio::test]
    async fn access_denials_do_not_allow_offline_fallback() {
        for status in [401, 403, 404, 500, 503] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                socket.read(&mut request).unwrap();
                write!(socket, "HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            });
            let error = reqwest::get(format!("http://{address}"))
                .await
                .unwrap()
                .error_for_status()
                .unwrap_err();
            let error = anyhow::Error::new(error)
                .context("Server rejected variable request");
            assert_eq!(allows_cache_fallback(&error), status >= 500);
            server.join().unwrap();
        }
        assert!(!allows_cache_fallback(&anyhow::anyhow!(
            "Decryption failed"
        )));
    }
}
