use super::*;
use crate::utils::{
    config::Config,
    messaging::{self, Client, CreatedLink, FriendCode, Link},
};
use reqwest::Method;
use serde_json::json;

/// Create a single-use friend code, or manage previously created codes
#[derive(Parser, Debug)]
pub struct Args {
    /// Restrict redemption to this stable user UUID
    pub target: Option<String>,
    #[arg(long, default_value = "24h")]
    pub expires: String,
    #[arg(long, conflicts_with_all = ["target", "revoke"])]
    pub list: bool,
    #[arg(long, conflicts_with = "target")]
    pub revoke: Option<String>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub before: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u16,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let client = Client::new(config)?;
    if let Some(id) = args.revoke {
        let id = uuid::Uuid::parse_str(&id)?;
        client.delete(&format!("/v2/friend-links/{id}")).await?;
        println!(
            "{}",
            if args.json {
                "{\"revoked\":true}"
            } else {
                "Friend link revoked."
            }
        );
        return Ok(());
    }
    if args.list {
        let links: Vec<Link> = client
            .request(
                Method::GET,
                &messaging::page_path(
                    "/v2/friend-links",
                    args.before.as_deref(),
                    args.limit,
                )?,
                None,
            )
            .await?;
        if args.json {
            println!("{}", serde_json::to_string(&links)?);
        } else {
            for link in links {
                let status = if let Some(who) = link.redeemed_by {
                    format!(
                        "redeemed by {} · {} · {}",
                        messaging::safe(&who.username),
                        who.id,
                        who.fingerprint
                    )
                } else if link.revoked_at.is_some() {
                    "revoked".into()
                } else if link.expires_at <= chrono::Utc::now() {
                    "expired".into()
                } else {
                    format!("pending · expires {}", link.expires_at)
                };
                println!(
                    "{} · {} · {}",
                    messaging::safe(&link.label),
                    link.id,
                    status
                );
            }
        }
        return Ok(());
    }
    let target = args
        .target
        .map(|id| uuid::Uuid::parse_str(&id).map(|id| id.to_string()))
        .transpose()?;
    let adjectives = [
        "amber", "quiet", "silver", "gentle", "bright", "misty", "violet",
        "sunny",
    ];
    let animals = [
        "otter", "maple", "wren", "fox", "cedar", "panda", "heron", "badger",
    ];
    use rand::seq::SliceRandom;
    let label = format!(
        "{}-{}",
        adjectives.choose(&mut rand::thread_rng()).unwrap(),
        animals.choose(&mut rand::thread_rng()).unwrap()
    );
    let created: CreatedLink = client.request(Method::POST, "/v2/friend-links", Some(json!({"label":label,"target_id":target,"expires_at":messaging::expires(&args.expires)?}))).await?;
    let code = FriendCode {
        version: 1,
        server: client.origin.clone(),
        id: created.link.id.clone(),
        creator_id: client.user_id().into(),
        creator_fingerprint: crate::utils::messaging_crypto::fingerprint(
            &client.key.key.public_key_str()?,
        )?,
        token: created.token,
    };
    // Record authorization before displaying a redeemable link.
    client.state.put("created-link", &created.link.id, &label)?;
    let code = format!(
        "{}:envx-friend-v1:{}",
        label,
        hex::encode(serde_json::to_vec(&code)?)
    );
    if args.json {
        println!("{}", json!({"code":code,"link":created.link}));
    } else {
        println!("{code}");
        eprintln!("Single use · expires {}", created.link.expires_at);
    }
    Ok(())
}
