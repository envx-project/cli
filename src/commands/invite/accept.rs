use super::*;
use crate::utils::{
    project_snapshot::{self, Client, InvitePayload, PreparedInvite, VERSION},
    symmetric::password_decrypt_from_armor,
};
use anyhow::bail;
use uuid::Uuid;
#[derive(Parser)]
pub struct Args {
    invite_code: String,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.unlocked_primary_key()?;
    let client = Client::new(config, &key)?;
    let [sym, code, verifier]: [Uuid; 3] = args
        .invite_code
        .split(':')
        .map(Uuid::parse_str)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| anyhow!("invalid invite code"))?;
    let prepared: PreparedInvite = client
        .json(
            "/v2/invite/prepare",
            &serde_json::json!({"code":code,"verifier":verifier}),
        )
        .await?;
    let plaintext =
        password_decrypt_from_armor(&prepared.ciphertext, &sym.to_string())?;
    let payload:InvitePayload=serde_json::from_str(&plaintext).context("Unsupported invitation payload; regenerate the invitation with the current envx version")?;
    if prepared.protocol_version != VERSION
        || payload.protocol_version != VERSION
        || prepared.invite_id != code.to_string()
        || prepared.project_id != payload.project_id
        || prepared.source_snapshot != payload.snapshot
    {
        bail!(
            "Invitation snapshot is inconsistent; regenerate the invitation."
        );
    }
    let variables = project_snapshot::rewrap(
        &payload.variables,
        &payload.project_id,
        &prepared.users,
        &key,
    )?;
    client.post("/v2/invite/accept",&serde_json::json!({"protocol_version":VERSION,"code":code,"verifier":verifier,"snapshot":prepared.snapshot,"variables":variables})).await?;
    println!("Successfully joined project {}", payload.project_id);
    Ok(())
}
