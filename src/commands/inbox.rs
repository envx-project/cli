use super::*;
use crate::utils::{
    config::Config,
    messaging::{self, Client, Message},
};
use reqwest::Method;

/// List message metadata without revealing secrets, or delete your mailbox copy
#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub before: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u16,
    #[arg(long)]
    pub delete: Option<String>,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let client = Client::new(config)?;
    if let Some(id) = args.delete {
        let id = uuid::Uuid::parse_str(&id)?;
        client.delete(&format!("/v2/messages/{id}")).await?;
        println!(
            "{}",
            if args.json {
                "{\"deleted\":true}"
            } else {
                "Deleted your mailbox copy."
            }
        );
        return Ok(());
    }
    let messages: Vec<Message> = client
        .request(
            Method::GET,
            &messaging::page_path(
                "/v2/messages",
                args.before.as_deref(),
                args.limit,
            )?,
            None,
        )
        .await?;
    if args.json {
        println!("{}", serde_json::to_string(&messages)?);
    } else if messages.is_empty() {
        println!("No messages.");
    } else {
        for message in messages {
            let (direction, peer) = if message.sender_id == client.user_id() {
                ("to", message.recipient_id)
            } else {
                ("from", message.sender_id)
            };
            println!(
                "{} · {} {} · {}{}",
                message.id,
                direction,
                peer,
                message.created_at,
                message
                    .expires_at
                    .map(|time| format!(" · expires {time}"))
                    .unwrap_or_default()
            );
        }
    }
    Ok(())
}
