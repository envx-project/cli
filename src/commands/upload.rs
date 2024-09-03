use super::*;
use crate::{
    sdk::SDK,
    utils::{config::get_config, prompt::prompt_text},
};

/// If your key is not in the database, use this command to upload it
#[derive(Parser)]
pub struct Args {
    /// Key to sign with
    #[clap(short, long)]
    key: String,

    /// Username to add to project
    #[clap(short, long)]
    username: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = get_config()?;

    let key = config.get_key(&args.key)?;

    let username = match args.username {
        Some(u) => u,
        None => prompt_text("Username: ")?,
    };

    let id = SDK::new_user(&username, &key.public_key()?).await?;
    println!("UUID: {}", &id);

    config.set_uuid(&key.fingerprint, &id)?;

    config.write()?;

    Ok(())
}
