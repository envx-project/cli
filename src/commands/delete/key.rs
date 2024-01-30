use super::*;
use crate::{
    sdk::SDK,
    utils::{
        key::Key,
        prompt::{prompt_confirm, prompt_multi_options},
    },
};
use anyhow::{bail, Context};

/// Delete a key (Caution, keys will still stay on the server for now)
#[derive(Debug, Parser)]
pub struct Args {
    /// Fingerprint of the key to delete
    #[clap(short, long)]
    key: Option<String>,

    /// Force deletion of primary key
    #[clap(short, long)]
    force: bool,
}

// TODOS
// TODO: fix configuration race condition while deleting multiple keys

pub async fn command(args: Args) -> Result<()> {
    let mut config = crate::utils::config::get_config().context("Failed to get config")?;
    let kl_arc = std::sync::Arc::new(&config.keys);
    let primary_key = &config.primary_key;

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

    if selected.contains(primary_key) {
        println!("You have selected your primary key for deletion.");
        println!("You will not be able to use envx until you set a new primary key.");

        if args.force {
            println!("Continuing because of --force");
        } else {
            let confirmation = prompt_confirm("Are you sure you want to continue?")?;

            if !confirmation {
                println!("Aborting...");
                return Ok(());
            }

            println!("Set a new primary key with `envx change primary-key`");
        }
    }

    println!("Deleting keys: {:?}", selected);

    let keys = selected
        .iter()
        .map(|it| find(it, &kl_arc).expect("Failed to find key (1)"))
        .collect::<Vec<_>>();

    let tasks: Vec<_> = keys
        .into_iter()
        .map(|item| -> tokio::task::JoinHandle<anyhow::Result<()>> {
            tokio::spawn(async move {
                let key_dir = crate::utils::rpgp::get_vault_location()?.join(&item.fingerprint);

                if item.uuid.is_some() {
                    println!("Deleting key {} on server...", &item);
                    match SDK::delete_key(&item.fingerprint).await {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Failed to delete key on server: {}", e);
                            bail!("Failed to delete key on server");
                        }
                    }
                } else {
                    println!("Key {} not on server", item);
                }

                if key_dir.exists() {
                    std::fs::remove_dir_all(key_dir).context("Failed to delete key directory")?
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
