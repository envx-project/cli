use super::*;
use crate::utils::{
    config::Config,
    messaging::{self, Client, Message},
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
#[derive(Parser, Debug)]
pub struct Args {
    #[arg(required_unless_present = "retry")]
    pub friend: Option<String>,
    #[arg(long, conflicts_with = "stdin")]
    pub file: Option<PathBuf>,
    #[arg(long)]
    pub stdin: bool,
    /// Parse input as KEY=VALUE lines instead of plain text
    #[arg(long)]
    pub env: bool,
    #[arg(long)]
    pub expires: Option<String>,
    #[arg(long)]
    pub json: bool,
    /// Retry a previously prepared send without creating a duplicate
    #[arg(long, conflicts_with_all = ["friend", "file", "stdin", "env", "expires"])]
    pub retry: Option<String>,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let client = Client::new(config)?;
    let (id, request) = if let Some(id) = args.retry {
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
        (id, body)
    } else {
        let target = args
            .friend
            .context("Specify a friend's UUID or local alias")?;
        let friend = client.resolve(&target).await?;
        let pin = client.trusted(&friend.user)?;
        let text = input(args.file, args.stdin)?;
        let payload = if args.env {
            Payload::Variables(parse_variables(&text)?)
        } else {
            Payload::Text(text)
        };
        let id = uuid::Uuid::new_v4().to_string();
        let expires_at = args
            .expires
            .as_deref()
            .map(messaging::expires)
            .transpose()?;
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
        (id, request)
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
        println!("Sent {}", sent.id);
    }
    Ok(())
}
fn input(file: Option<PathBuf>, stdin: bool) -> Result<String> {
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
        return inquire::Password::new("Secret:")
            .without_confirmation()
            .with_render_config(crate::utils::prompt::get_render_config())
            .with_help_message(
                "Hidden input. Use --stdin or --file for multiline secrets.",
            )
            .prompt()
            .context("Failed to read secret");
    }
    if bytes.len() > 65536 {
        anyhow::bail!("Secret input exceeds 64 KiB");
    }
    if bytes.is_empty() {
        anyhow::bail!("Secret input is empty");
    }
    String::from_utf8(bytes).context("Secret input must be UTF-8 text")
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
}
