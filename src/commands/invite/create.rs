use super::*;
use crate::utils::{
    choice::Choice,
    project_snapshot::{self, Client, InvitePayload, VERSION},
    symmetric::password_encrypt_to_armor,
};
use uuid::Uuid;
#[derive(Parser)]
pub struct Args {
    /// Project ID
    #[arg(short, long)]
    project_id: Option<String>,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.unlocked_primary_key()?;
    let client = Client::new(config, &key)?;
    let project_id = Choice::try_project(config, args.project_id, &key).await?;
    let snapshot = client.snapshot(&project_id, &[]).await?;
    let variables = project_snapshot::decrypt(&snapshot, &key)?;
    let payload = InvitePayload {
        protocol_version: VERSION,
        project_id: project_id.clone(),
        snapshot: snapshot.snapshot.clone(),
        variables,
    };
    let sym = Uuid::new_v4().to_string();
    let encrypted =
        password_encrypt_to_armor(&serde_json::to_string(&payload)?, &sym)?;
    #[derive(serde::Deserialize)]
    struct Created {
        invite_code: String,
        verifier: String,
    }
    let response:Created=client.json("/v2/invite/new",&serde_json::json!({"protocol_version":VERSION,"project_id":project_id,"snapshot":snapshot.snapshot,"ciphertext":encrypted})).await?;
    let code = format!("{sym}:{}:{}", response.invite_code, response.verifier);
    if args.json {
        println!(
            "{}",
            serde_json::json!({"code":code,"project_id":project_id})
        );
    } else {
        println!("envx invite accept {code}");
    }
    Ok(())
}
