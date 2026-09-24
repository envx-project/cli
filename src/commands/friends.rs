use super::*;
use crate::utils::{
    config::Config,
    messaging::{self, Client, Pin},
};
use serde_json::json;

/// List friends, assign local aliases, verify keys, or remove a friendship
#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    pub json: bool,
    /// Friend UUID or local alias to remove
    #[arg(long, conflicts_with_all = ["rename", "accept_key"])]
    pub remove: Option<String>,
    /// Friend UUID or local alias to rename on this machine
    #[arg(long, requires = "alias", conflicts_with = "accept_key")]
    pub rename: Option<String>,
    #[arg(long, requires = "rename")]
    pub alias: Option<String>,
    /// Explicitly trust this friend's key after verifying its fingerprint
    #[arg(long, requires = "fingerprint")]
    pub accept_key: Option<String>,
    #[arg(long, requires = "accept_key")]
    pub fingerprint: Option<String>,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let interactive = !args.json
        && args.remove.is_none()
        && args.rename.is_none()
        && args.accept_key.is_none()
        && crate::utils::prompt::is_interactive();
    let client = Client::new(config)?;
    if let Some(target) = args.remove {
        let friend = client.resolve(&target).await?;
        client
            .delete(&format!("/v2/friends/{}", friend.user.id))
            .await?;
        // Keep trust pins for historical messages and to detect a future key change.
        println!(
            "{}",
            if args.json {
                "{\"removed\":true}"
            } else {
                "Friend removed. Existing messages remain available."
            }
        );
        return Ok(());
    }
    if let Some(target) = args.rename {
        let friend = client.resolve(&target).await?;
        let alias = args.alias.context("Missing alias")?;
        if alias.trim().is_empty()
            || alias.chars().any(char::is_control)
            || alias.len() > 100
            || uuid::Uuid::parse_str(&alias).is_ok()
        {
            anyhow::bail!("Alias must be 1–100 characters, contain no controls, and not be a UUID");
        }
        for (_, pin) in client.state.list::<Pin>("friend")? {
            if pin.user_id != friend.user.id
                && pin.alias.as_deref() == Some(&alias)
            {
                anyhow::bail!("Alias already belongs to another friend");
            }
        }
        client.trusted(&friend.user)?;
        client.pin(&friend.user, Some(alias), false)?;
    }
    if let Some(target) = args.accept_key {
        let friend = client.resolve(&target).await?;
        if args
            .fingerprint
            .as_deref()
            .map(str::to_lowercase)
            .as_deref()
            != Some(friend.user.fingerprint.as_str())
        {
            anyhow::bail!(
                "Fingerprint does not match the friend's current key"
            );
        }
        let old: Option<Pin> = client.state.get("friend", &friend.user.id)?;
        client.pin(&friend.user, old.and_then(|p| p.alias), true)?;
    }
    let friends = client.friends().await?;
    let mut rows = Vec::new();
    for friend in &friends {
        client.learn_from_receipt(friend)?;
        let pin: Option<Pin> = client.state.get("friend", &friend.user.id)?;
        let status = match &pin {
            Some(pin) if pin.fingerprint == friend.user.fingerprint => {
                "trusted"
            }
            Some(_) => "KEY CHANGED",
            None => "unverified",
        };
        let alias = pin.and_then(|p| p.alias);
        if !args.json {
            println!(
                "{} · {} · {} · {}",
                messaging::safe(
                    alias.as_deref().unwrap_or(&friend.user.username)
                ),
                friend.user.id,
                friend.user.fingerprint,
                status
            );
        }
        rows.push(json!({"user":friend.user,"alias":alias,"trust":status,"created_at":friend.created_at}));
    }
    if args.json {
        println!("{}", serde_json::to_string(&rows)?);
    } else if rows.is_empty() {
        println!(
            "No friends yet. Share an `envx friend-link` code to add one."
        );
    }
    if interactive && !friends.is_empty() {
        let mut options =
            vec!["Done".to_owned(), "Create a friend link".to_owned()];
        for friend in &friends {
            let pin: Option<Pin> =
                client.state.get("friend", &friend.user.id)?;
            let label = pin
                .as_ref()
                .and_then(|pin| pin.alias.as_deref())
                .unwrap_or(&friend.user.username);
            options.push(format!(
                "{} · {}",
                messaging::safe(label),
                friend.user.id
            ));
        }
        let selection =
            inquire::Select::new("Friend options", options).prompt()?;
        if selection == "Create a friend link" {
            super::friend_link::command(
                super::friend_link::Args {
                    target: None,
                    expires: "24h".into(),
                    list: false,
                    revoke: None,
                    json: false,
                    before: None,
                    limit: 50,
                },
                config,
            )
            .await?;
        } else if selection != "Done" {
            let target = selection
                .rsplit_once(" · ")
                .context("Invalid friend selection")?
                .1
                .to_owned();
            let action = inquire::Select::new(
                "Action",
                vec![
                    "Send secret",
                    "Show fingerprint",
                    "Rename locally",
                    "Remove friend",
                    "Cancel",
                ],
            )
            .prompt()?;
            match action {
                "Send secret" => {
                    super::send::command(
                        super::send::Args {
                            friend: Some(target),
                            file: None,
                            stdin: false,
                            env: false,
                            expires: None,
                            json: false,
                            retry: None,
                        },
                        config,
                    )
                    .await?
                }
                "Show fingerprint" => {
                    let friend = client.resolve(&target).await?;
                    println!(
                        "{} · {}",
                        friend.user.id, friend.user.fingerprint
                    );
                }
                "Rename locally" => {
                    let alias =
                        crate::utils::prompt::prompt_text("Local alias:")?;
                    Box::pin(command(
                        Args {
                            json: false,
                            remove: None,
                            rename: Some(target),
                            alias: Some(alias),
                            accept_key: None,
                            fingerprint: None,
                        },
                        config,
                    ))
                    .await?;
                }
                "Remove friend"
                    if crate::utils::prompt::prompt_confirm(
                        "Remove this friendship for both users?",
                    )? =>
                {
                    client.delete(&format!("/v2/friends/{target}")).await?;
                    println!("Friend removed.");
                }
                _ => {}
            }
        }
    }
    Ok(())
}
