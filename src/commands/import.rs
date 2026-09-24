use super::*;
use crate::utils::{
    config::Config, key::Key, messaging, messaging_crypto,
    rpgp::get_vault_location, state::StateStore,
};
use clap::Subcommand;
use pgp::{composed::ArmorOptions, types::KeyDetails};
use std::{fs, io::Write};

/// Import a public key or named variables from a secret message
#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Store a public key locally without changing your primary signing key
    Pubkey { path: String },
    /// Import named variables from a verified encrypted message
    Message(super::import_message::Args),
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    match args.command {
        Commands::Message(args) => {
            super::import_message::command(args, config).await
        }
        Commands::Pubkey { path } => {
            let public = messaging_crypto::public_key(
                &fs::read_to_string(path)
                    .context("Failed to read public key file")?,
            )?;
            let fingerprint = hex::encode(public.fingerprint().as_bytes());
            let primary_user_id = public
                .details
                .users
                .first()
                .map(|user| String::from_utf8_lossy(user.id.id()).into_owned())
                .unwrap_or_else(|| "Unnamed public key".into());
            let directory = get_vault_location()?.join(&fingerprint);
            fs::create_dir_all(&directory)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    &directory,
                    fs::Permissions::from_mode(0o700),
                )?;
            }
            let path = directory.join("public.key");
            let armor = public.to_armored_string(ArmorOptions::default())?;
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&path) {
                Ok(mut file) => {
                    file.write_all(armor.as_bytes())?;
                    file.sync_all()?;
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists =>
                {
                    if messaging_crypto::fingerprint(&fs::read_to_string(
                        &path,
                    )?)? != fingerprint
                    {
                        anyhow::bail!("Existing public key does not match; refusing to overwrite it");
                    }
                }
                Err(error) => return Err(error.into()),
            }
            let key = Key {
                fingerprint: fingerprint.clone(),
                note: "Imported public key".into(),
                primary_user_id,
                pubkey_only: Some(true),
                uuid: None,
            };
            StateStore::open(config)?.put(
                "imported-public-key",
                &fingerprint,
                &key,
            )?;
            println!(
                "Imported {} · {}",
                fingerprint,
                messaging::safe(&key.primary_user_id)
            );
            Ok(())
        }
    }
}
