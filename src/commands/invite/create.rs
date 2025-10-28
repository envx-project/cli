use envx_sdk::models::InviteBody;
use uuid::Uuid;

use crate::{
    sdk::SDK,
    utils::{choice::Choice, symmetric::password_encrypt_to_armor},
};

use super::*;

#[derive(Parser)]
pub struct Args {
    /// Project ID
    #[clap(short, long)]
    project_id: Option<String>,

    /// Output as JSON
    #[clap(long)]
    json: bool,
}

pub async fn command(args: Args, config: Config) -> Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let key = key.unlock(&password);
    let sdk_config = config.sdk_configuration(&key)?;

    let project_id = Choice::try_project(args.project_id, &key).await?;
    let kvpairs = SDK::get_variables(&project_id, &key).await?;
    let stringified_kvpairs = serde_json::to_string(&kvpairs)?;

    let symmetrical_encryption_key = uuid::Uuid::new_v4().to_string();
    let encrypted = password_encrypt_to_armor(
        &stringified_kvpairs,
        &symmetrical_encryption_key,
    )?;

    let response = envx_sdk::apis::invite_api::new_invite(
        &sdk_config,
        InviteBody {
            ciphertext: encrypted,
            project_id: Uuid::parse_str(&project_id)?,
        },
    )
    .await?;

    println!(
        "envx invite accept {}:{}:{}",
        symmetrical_encryption_key, response.invite_code, response.verifier
    );

    Ok(())
}
