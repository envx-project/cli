use super::*;
use crate::utils::{
    config::Config,
    messaging::{self, Client, FriendCode, LinkResult},
    messaging_crypto::{self as crypto, Receipt},
};
use reqwest::Method;
use serde_json::json;

/// Redeem a single-use friend code and pin its creator's public key
#[derive(Parser, Debug)]
pub struct Args {
    pub code: String,
    #[arg(long)]
    pub alias: Option<String>,
    #[arg(long)]
    pub json: bool,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let (_, encoded) = args
        .code
        .split_once(":envx-friend-v1:")
        .context("Invalid friend code")?;
    if encoded.len() > 8192 {
        anyhow::bail!("Friend code is too large");
    }
    let code: FriendCode = serde_json::from_slice(&hex::decode(encoded)?)?;
    if code.version != 1 {
        anyhow::bail!("Unsupported friend code version");
    }
    uuid::Uuid::parse_str(&code.id)?;
    uuid::Uuid::parse_str(&code.creator_id)?;
    let client = Client::new(config)?;
    if messaging::normalized_origin(&code.server)? != client.origin {
        anyhow::bail!("Friend code belongs to another server; select that server explicitly before redeeming");
    }
    if code.creator_id == client.user_id() {
        anyhow::bail!("You cannot add yourself");
    }
    let preview: LinkResult = client
        .request(
            Method::POST,
            "/v2/friend-links/preview",
            Some(json!({"id":code.id,"token":code.token})),
        )
        .await?;
    check_creator(&code, &preview)?;
    if let Some(target) = &preview.link.target_id {
        if target != client.user_id() {
            anyhow::bail!("This friend code is intended for another user");
        }
    }
    let receipt = Receipt {
        version: 1,
        id: code.id.clone(),
        creator_id: code.creator_id.clone(),
        creator_fingerprint: code.creator_fingerprint.clone(),
        redeemer_id: client.user_id().into(),
        redeemer_fingerprint: crypto::fingerprint(
            &client.key.key.public_key_str()?,
        )?,
    };
    let signed =
        match client.state.get::<String>("pending-redemption", &code.id)? {
            Some(signed) => signed,
            None => {
                let signed = crypto::sign(&receipt, &client.key)?;
                client.state.put("pending-redemption", &code.id, &signed)?;
                signed
            }
        };
    let result: LinkResult = client
        .request(
            Method::POST,
            "/v2/friend-links/redeem",
            Some(json!({"id":code.id,"token":code.token,"receipt":signed})),
        )
        .await?;
    check_creator(&code, &result)?;
    let pin = client.pin(&result.creator, args.alias, false)?;
    client.state.delete("pending-redemption", &code.id)?;
    if args.json {
        println!("{}", serde_json::to_string(&pin)?);
    } else {
        println!(
            "Added {} · {} · {}",
            messaging::safe(&result.creator.username),
            pin.user_id,
            pin.fingerprint
        );
    }
    Ok(())
}
fn check_creator(code: &FriendCode, result: &LinkResult) -> Result<()> {
    messaging::validate_identity(&result.creator)?;
    if result.link.id != code.id
        || result.link.creator_id != code.creator_id
        || result.creator.id != code.creator_id
        || result.creator.fingerprint != code.creator_fingerprint
    {
        anyhow::bail!("Friend code identity verification failed");
    }
    Ok(())
}
