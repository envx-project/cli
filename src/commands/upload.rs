use super::*;
use crate::{
    sdk::SDK,
    utils::{config::Config, prompt::prompt_text},
};

/// If your key is not in the database, use this command to upload it
#[derive(Parser)]
pub struct Args {
    /// Username to add to project
    #[arg(short, long)]
    username: Option<String>,
}

// TODO: probably irrelevant and should be removed
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;

    let username = match args.username {
        Some(u) => u,
        None => prompt_text("Username: ")?,
    };

    let id = SDK::new_user(&username, &key.public_key_str()?).await?;
    println!("UUID: {}", &id);

    config.set_uuid(&key.fingerprint, &id)?;

    Ok(())
}
