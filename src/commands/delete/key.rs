// TODO: also delete keys on the server side

use std::thread::JoinHandle;

use crate::{
    sdk::SDK,
    utils::{key::Key, prompt::prompt_multi_options},
};

use super::*;
use anyhow::Context;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

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
    let key_list = config.keys.clone();
    let kl_arc = std::sync::Arc::new(key_list);

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

    let tasks: Vec<_> = selected
        .into_iter()
        .map(|mut item| -> tokio::task::JoinHandle<anyhow::Result<()>> {
            tokio::spawn(async {
                // let key = find(&item, &key_list).expect("Failed to find key");
                // let key = kl_arc
                //     .iter()
                //     .find(|k| k.fingerprint == item)
                //     .expect("Failed to find key")
                //     .clone();

                // if key.uuid.is_some() {
                //     println!("Deleting key {} on server", key);
                //     SDK::delete_key(&key.fingerprint).await.unwrap();
                // } else {
                //     println!("Key {} not on server", key);
                // }
                //
                let vault_location = crate::utils::rpgp::get_vault_location()?;
                // let key_dir = vault_location.join(&key.fingerprint);
                // if key_dir.exists() {
                //     std::fs::remove_dir_all(key_dir)
                //         .context("Failed to delete key directory")
                //         .unwrap();
                // } else {
                //     println!("Key {} not on disk", key);
                // }
                //
                // item;

                Ok(())
            })
        })
        .collect();

    // selected.iter().for_each(|key| {});

    // &selected.par_iter().for_each(|key| async {
    //     let key = find(&key, &key_list).expect("Failed to find key");
    //
    //     if key.uuid.is_some() {
    //         // println!("Deleting key {} on server", key);
    //         SDK::delete_key(&key.fingerprint).await.unwrap();
    //     } else {
    //         // println!("Key {} not on server", key);
    //     }
    //
    //     let key_dir = vault_location.join(&key.fingerprint);
    //     if key_dir.exists() {
    //         std::fs::remove_dir_all(key_dir)
    //             .context("Failed to delete key directory")
    //             .unwrap();
    //     } else {
    //         // println!("Key {} not on disk", key);
    //     }
    // });

    // for key in &selected {
    //     let key = find(&key, &key_list).expect("Failed to find key");
    //
    //     if key.uuid.is_some() {
    //         // println!("Deleting key {} on server", key);
    //         SDK::delete_key(&key.fingerprint).await?;
    //     } else {
    //         // println!("Key {} not on server", key);
    //     }
    //
    //     let key_dir = vault_location.join(&key.fingerprint);
    //     if key_dir.exists() {
    //         std::fs::remove_dir_all(key_dir).context("Failed to delete key directory")?;
    //     } else {
    //         // println!("Key {} not on disk", key);
    //     }
    // }

    config.keys.retain(|k| !&selected.contains(&k.fingerprint));

    config.write().context("Failed to write config")?;

    Ok(())
}

fn find(key: &str, keys: &[Key]) -> Option<Key> {
    keys.iter().find(|k| k.fingerprint == key).cloned()
}
