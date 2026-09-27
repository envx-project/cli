use super::*;
use crate::utils::{
    config::Config,
    messaging::{self, Client, Friend, Message},
    messaging_crypto::{self as crypto, Envelope, Payload},
};
use reqwest::Method;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{IsTerminal, Read},
    path::PathBuf,
};

/// Send a signed, encrypted secret to a friend; values never go in arguments
///
/// Run without arguments in a terminal to pick a friend and secret
/// interactively. Piped input is read automatically; KEY=VALUE lines are
/// sent as variables unless --text is given.
#[derive(Parser, Debug)]
pub struct Args {
    /// Friend UUID or local alias (prompted for when omitted in a terminal)
    pub friend: Option<String>,
    /// Read the secret from a file
    #[arg(long, conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    /// Read the secret from stdin even when it is a terminal
    #[arg(long)]
    pub stdin: bool,
    /// Parse input as KEY=VALUE lines instead of plain text
    #[arg(long, conflicts_with = "text")]
    pub env: bool,
    /// Send input as plain text even if it looks like KEY=VALUE lines
    #[arg(long)]
    pub text: bool,
    /// Expire the message after a duration such as 30m, 24h, or 7d
    #[arg(long)]
    pub expires: Option<String>,
    #[arg(long)]
    pub json: bool,
    /// Retry a previously prepared send without creating a duplicate
    #[arg(long, conflicts_with_all = ["friend", "file", "stdin", "env", "text", "expires"])]
    pub retry: Option<String>,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let client = Client::new(config)?;
    let (id, request, recipient_name) = if let Some(id) = args.retry {
        let id = uuid::Uuid::parse_str(&id)?.to_string();
        let body: Value = client
            .state
            .get("pending-send", &id)?
            .context("No pending send with that ID")?;
        let recipient = body
            .get("recipient_id")
            .and_then(Value::as_str)
            .context("Invalid pending recipient")?;
        let friend = client.resolve(recipient).await?;
        let pin = client.trusted(&friend.user)?;
        let ciphertext = body
            .get("ciphertext")
            .and_then(Value::as_str)
            .context("Invalid pending ciphertext")?;
        let envelope = crypto::decrypt(
            ciphertext,
            &client.key,
            &client.key.key.public_key_str()?,
        )?;
        if envelope.server != client.origin
            || envelope.id != id
            || envelope.sender_id != client.user_id()
            || envelope.recipient_id != recipient
            || envelope.recipient_fingerprint != pin.fingerprint
        {
            anyhow::bail!(
                "Pending send identity no longer matches trusted friend"
            );
        }
        let names =
            crate::utils::user_display::UserDisplay::from_state(&client.state)?;
        (id, body, names.name(&friend.user.id, &friend.user.username))
    } else {
        // Wizard only when nothing was specified and a human can answer.
        let wizard = args.friend.is_none()
            && args.file.is_none()
            && !args.stdin
            && crate::utils::prompt::is_interactive();
        let friend = match args.friend {
            Some(target) => client.resolve(&target).await?,
            None if wizard => select_friend(&client).await?,
            None => anyhow::bail!(
                "Specify a friend's UUID or local alias, or run `envx send` in a terminal to choose one"
            ),
        };
        let pin = client.trusted(&friend.user)?;
        let (file, expires) = if wizard {
            let file = prompt_source()?;
            let expires = match args.expires {
                Some(expires) => Some(expires),
                None => prompt_expiry()?,
            };
            (file, expires)
        } else {
            (args.file, args.expires)
        };
        // Validate expiry before asking for the secret.
        let expires_at =
            expires.as_deref().map(messaging::expires).transpose()?;
        let (text, typed) = input(file, args.stdin)?;
        let payload = if args.env {
            Payload::Variables(parse_variables(&text)?)
        } else if args.text || typed {
            Payload::Text(text)
        } else if let Some(variables) = detect_variables(&text) {
            eprintln!(
                "Detected {} KEY=VALUE variable{}; pass --text to send as plain text.",
                variables.len(),
                if variables.len() == 1 { "" } else { "s" }
            );
            Payload::Variables(variables)
        } else {
            Payload::Text(text)
        };
        let id = uuid::Uuid::new_v4().to_string();
        let envelope = Envelope {
            version: 1,
            server: client.origin.clone(),
            id: id.clone(),
            sender_id: client.user_id().into(),
            recipient_id: friend.user.id.clone(),
            sender_fingerprint: crypto::fingerprint(
                &client.key.key.public_key_str()?,
            )?,
            recipient_fingerprint: pin.fingerprint,
            expires_at,
            payload,
        };
        let ciphertext =
            crypto::encrypt(&envelope, &client.key, &pin.public_key)?;
        if ciphertext.len() > 128 * 1024 {
            anyhow::bail!(
                "Encrypted message exceeds 128 KiB; send a smaller secret"
            );
        }
        let request = json!({"id":id,"recipient_id":friend.user.id,"ciphertext":ciphertext,"expires_at":expires_at});
        client.state.put("pending-send", &id, &request)?;
        let names =
            crate::utils::user_display::UserDisplay::from_state(&client.state)?;
        (
            id,
            request,
            names.name(&friend.user.id, &friend.user.username),
        )
    };
    let sent = client
        .request::<Message>(Method::POST, "/v2/messages", Some(request))
        .await;
    let sent = match sent {
        Ok(sent) => sent,
        Err(error) => {
            eprintln!("Retry safely with: envx send --retry {id}");
            return Err(error);
        }
    };
    client.state.delete("pending-send", &id)?;
    if args.json {
        println!("{}", serde_json::to_string(&sent)?);
    } else {
        println!("Sent to {recipient_name}.\n  envx read {id}");
    }
    Ok(())
}
/// Presentation wrapper so the picker shows names while returning the friend.
struct Choice<T>(String, T);
impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

async fn select_friend(client: &Client) -> Result<Friend> {
    let names =
        crate::utils::user_display::UserDisplay::from_state(&client.state)?;
    let mut choices = Vec::new();
    for friend in client.friends().await? {
        client.learn_from_receipt(&friend)?;
        // Only offer friends whose current key matches the local pin.
        if client.trusted(&friend.user).is_ok() {
            choices.push(Choice(
                names.row(&friend.user.id, &friend.user.username, false),
                friend,
            ));
        }
    }
    if choices.is_empty() {
        anyhow::bail!(
            "No trusted friends to send to. Share an `envx friend-link` code, or verify a friend with `envx friends --accept-key`."
        );
    }
    Ok(inquire::Select::new("Send to:", choices)
        .with_render_config(crate::utils::prompt::get_render_config())
        .prompt()
        .context("Failed to choose a friend")?
        .1)
}

/// None means type the secret at a hidden prompt.
fn prompt_source() -> Result<Option<PathBuf>> {
    let choices = vec![
        Choice("Type a secret (hidden)".into(), false),
        Choice(
            "Send a file (KEY=VALUE files become variables)".into(),
            true,
        ),
    ];
    let from_file = inquire::Select::new("What to send:", choices)
        .with_render_config(crate::utils::prompt::get_render_config())
        .prompt()
        .context("Failed to choose what to send")?
        .1;
    if !from_file {
        return Ok(None);
    }
    let path = inquire::Text::new("File path:")
        .with_render_config(crate::utils::prompt::get_render_config())
        .with_validator(|path: &str| {
            Ok(if std::path::Path::new(path.trim()).is_file() {
                inquire::validator::Validation::Valid
            } else {
                inquire::validator::Validation::Invalid(
                    "No file at that path".into(),
                )
            })
        })
        .prompt()
        .context("Failed to read file path")?;
    Ok(Some(PathBuf::from(path.trim())))
}

fn prompt_expiry() -> Result<Option<String>> {
    let choices = vec![
        Choice("Never".into(), None),
        Choice("1 hour".into(), Some("1h")),
        Choice("24 hours".into(), Some("24h")),
        Choice("7 days".into(), Some("7d")),
    ];
    Ok(inquire::Select::new("Expires:", choices)
        .with_render_config(crate::utils::prompt::get_render_config())
        .prompt()
        .context("Failed to choose an expiry")?
        .1
        .map(Into::into))
}

/// Returns the secret and whether it was typed at the hidden prompt.
fn input(file: Option<PathBuf>, stdin: bool) -> Result<(String, bool)> {
    let mut bytes = Vec::new();
    if let Some(path) = file {
        std::fs::File::open(path)?
            .take(65537)
            .read_to_end(&mut bytes)?;
    } else if stdin || !std::io::stdin().is_terminal() {
        std::io::stdin()
            .lock()
            .take(65537)
            .read_to_end(&mut bytes)?;
    } else {
        let secret = inquire::Password::new("Secret:")
            .without_confirmation()
            .with_render_config(crate::utils::prompt::get_render_config())
            .with_help_message(
                "Hidden input. Pipe input or use --file for multiline secrets.",
            )
            .prompt()
            .context("Failed to read secret")?;
        return Ok((secret, true));
    }
    if bytes.len() > 65536 {
        anyhow::bail!("Secret input exceeds 64 KiB");
    }
    if bytes.is_empty() {
        anyhow::bail!("Secret input is empty");
    }
    let text =
        String::from_utf8(bytes).context("Secret input must be UTF-8 text")?;
    Ok((text, false))
}

/// Conservative: every line must look like a conventional env assignment,
/// so a bare token such as base64 with `=` padding stays plain text.
fn detect_variables(text: &str) -> Option<BTreeMap<String, String>> {
    let conventional = text.lines().all(|line| {
        let line = line.trim_start();
        if line.trim().is_empty() || line.starts_with('#') {
            return true;
        }
        line.split_once('=').is_some_and(|(name, value)| {
            name.bytes().any(|b| b.is_ascii_uppercase())
                && name.bytes().all(|b| {
                    b == b'_' || b.is_ascii_uppercase() || b.is_ascii_digit()
                })
                && !value.starts_with('=')
        })
    });
    conventional.then(|| parse_variables(text).ok()).flatten()
}
pub fn parse_variables(text: &str) -> Result<BTreeMap<String, String>> {
    let mut variables = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=').with_context(|| {
            format!("Expected KEY=VALUE on line {}", index + 1)
        })?;
        if name.is_empty()
            || !name.bytes().enumerate().all(|(i, b)| {
                b == b'_'
                    || b.is_ascii_alphabetic()
                    || (i > 0 && b.is_ascii_digit())
            })
        {
            anyhow::bail!("Invalid variable name on line {}", index + 1);
        }
        if variables.insert(name.into(), value.into()).is_some() {
            anyhow::bail!("Duplicate variable name on line {}", index + 1);
        }
    }
    if variables.is_empty() {
        anyhow::bail!("No variables found");
    }
    Ok(variables)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn env_parser_preserves_values_and_rejects_ambiguous_names_without_echoing_secrets(
    ) {
        let values =
            parse_variables("# comment\nTOKEN=a=b \nEMPTY=\n").unwrap();
        assert_eq!(values["TOKEN"], "a=b ");
        assert_eq!(values["EMPTY"], "");
        for value in [
            "A=private-value\nA=other",
            "1BAD=private-value",
            "private-value",
            "BAD NAME=private-value",
        ] {
            let error = parse_variables(value).unwrap_err().to_string();
            assert!(!error.contains("private-value"));
        }
    }

    #[test]
    fn detects_only_conventional_env_input_as_variables() {
        let values =
            detect_variables("# comment\nTOKEN=a=b\n\nAPI_KEY_2=\n").unwrap();
        assert_eq!(values["TOKEN"], "a=b");
        assert_eq!(values["API_KEY_2"], "");
        for text in [
            "hunter2",
            "QUJDRA==",
            "password=secret",
            "TOKEN=a\nplain line",
            "export TOKEN=a",
            "A=1\nA=2",
            "_=1",
            "",
        ] {
            assert!(detect_variables(text).is_none(), "{text:?}");
        }
    }
}
