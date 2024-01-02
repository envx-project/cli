// TODO: also delete keys on the server side
use crate::{
    sdk::SDK,
    utils::{key::Key, prompt::prompt_multi_options},
};

use super::*;
use anyhow::Context;

/// Delete a key (Caution, keys will still stay on the server for now)
#[derive(Debug, Parser)]
pub struct Args {
    /// Fingerprint of the key to delete
    #[clap(short, long)]
    key: Option<String>,
}

// TODOS
// TODO: fix configuration race condition while deleting multiple keys

pub async fn command(args: Args) -> Result<()> {
    let mut config = crate::utils::config::get_config().context("Failed to get config")?;
    let kl_arc = std::sync::Arc::new(&config.keys);

    let selected: Vec<String> = match args.key {
        Some(key) => {
            let key = config.get_key(&key).context("Failed to get key")?;
            vec![key.fingerprint.clone()]
        }

        None => prompt_multi_options("Select keys to delete", config.keys.clone())?
            .iter()
            .map(|k| k.fingerprint.clone())
            .collect(),
    };

    let selected = selected
        .iter()
        .map(|s| {
            // split at " - " and take the first part
            s.split(" - ")
                .next()
                .expect("Failed to split fingerprint")
                .trim()
                .to_string()
        })
        .collect::<Vec<_>>();

    println!("Deleting keys: {:?}", selected);

    let keys = selected
        .iter()
        .map(|it| {
            let key = find(it, &kl_arc).expect("Failed to find key (1)");
            key
        })
        .collect::<Vec<_>>();

    let tasks: Vec<_> = keys
        .into_iter()
        .map(|item| -> tokio::task::JoinHandle<anyhow::Result<()>> {
            tokio::spawn(async move {
                let key_dir = crate::utils::rpgp::get_vault_location()?.join(&item.fingerprint);

                if item.uuid.is_some() {
                    println!("Deleting key {} on server", &item);
                    SDK::delete_key(&item.fingerprint).await?;
                } else {
                    println!("Key {} not on server", item);
                }

                if key_dir.exists() {
                    std::fs::remove_dir_all(key_dir)
                        .context("Failed to delete key directory")
                        .unwrap();
                } else {
                    println!("Key {} not on disk", item);
                }

                Ok(())
            })
        })
        .collect();

    let results = futures::future::join_all(tasks).await;
    for result in results {
        result
            .context("Failed to join thread")?
            .context("Failed to delete key")?;
    }

    config.keys.retain(|k| !&selected.contains(&k.fingerprint));

    config.write().context("Failed to write config")?;

    Ok(())
}

fn find(key: &str, keys: &[Key]) -> Option<Key> {
    keys.iter().find(|k| k.fingerprint == key).cloned()
}
