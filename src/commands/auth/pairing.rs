use super::{channel::Handshake, identity::Bundle};
use crate::utils::{
    config::Config, key::UnlockedKey, prompt::require_interactive,
};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

const PREFIX: &str = "envx-pair-v1=";
#[derive(Deserialize)]
struct Created {
    id: String,
}
#[derive(Deserialize)]
struct Reply {
    phase: u8,
    message: Option<String>,
}
struct Relay<'a> {
    http: reqwest::Client,
    base: Url,
    id: String,
    token: Option<String>,
    source_key: Option<&'a UnlockedKey>,
    auth: std::sync::Mutex<Option<(std::time::Instant, String)>>,
}
fn validate_server(url: &Url) -> Result<()> {
    let loopback =
        matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && loopback))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
    {
        bail!("Pairing requires HTTPS (HTTP is allowed only on loopback)");
    }
    Ok(())
}
fn parse_link(link: &str) -> Result<(Url, String)> {
    let mut base = Url::parse(link).context("Invalid pairing link")?;
    validate_server(&base)?;
    let id = base
        .fragment()
        .and_then(|f| f.strip_prefix(PREFIX))
        .context("Not an envx pairing link")?;
    let id = Uuid::parse_str(id)
        .context("Invalid pairing identifier")?
        .to_string();
    base.set_fragment(None);
    Ok((base, id))
}
impl<'a> Relay<'a> {
    fn new(
        base: Url,
        id: String,
        token: Option<String>,
        source_key: Option<&'a UnlockedKey>,
    ) -> Result<Self> {
        validate_server(&base)?;
        Ok(Self {
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .build()?,
            base,
            id,
            token,
            source_key,
            auth: std::sync::Mutex::new(None),
        })
    }
    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T> {
        let mut request = self
            .http
            .post(format!(
                "{}/{}",
                self.base.as_str().trim_end_matches('/'),
                path
            ))
            .json(&body);
        if let Some(key) = self.source_key {
            let mut cached = self.auth.lock().map_err(|_| {
                anyhow::anyhow!("Pairing authentication lock failed")
            })?;
            let critical = matches!(
                body.get("action").and_then(|v| v.as_str()),
                Some("payload" | "cancel")
            );
            if critical
                || cached.as_ref().is_none_or(|(created, _)| {
                    created.elapsed() >= Duration::from_secs(60)
                })
            {
                *cached = Some((
                    std::time::Instant::now(),
                    key.auth_token()?.to_string(),
                ));
            }
            request = request.bearer_auth(
                &cached.as_ref().context("Missing pairing authentication")?.1,
            );
        }
        let mut response = request
            .send()
            .await
            .context("Pairing server could not be reached")?;
        if !response.status().is_success() {
            bail!("Pairing request failed ({}). The pairing may have expired, been cancelled, or already been claimed.",response.status());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > 150000 {
                bail!("Oversized pairing response");
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .context("Invalid pairing server response")
    }
    async fn exchange(
        &self,
        action: &str,
        message: Option<String>,
    ) -> Result<Reply> {
        self.post(
            &format!(
                "v2/auth/pairing/{}/{}",
                self.id,
                if self.source_key.is_some() {
                    "source"
                } else {
                    "receiver"
                }
            ),
            json!({"action":action,"message":message,"token":self.token}),
        )
        .await
    }
    async fn wait(&self, phase: u8) -> Result<String> {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            let reply = self.exchange("poll", None).await?;
            if reply.phase == phase {
                return reply.message.context("Missing pairing message");
            }
            if reply.phase > phase {
                bail!("Unexpected pairing state");
            }
        }
    }
    async fn bounded<T>(
        &self,
        work: impl std::future::Future<Output = Result<T>>,
    ) -> Result<T> {
        let result = tokio::select! {
            result=tokio::time::timeout(Duration::from_secs(600),work)=>result.context("Pairing expired; start again").and_then(|result| result),
            _=tokio::signal::ctrl_c()=>Err(anyhow::anyhow!("Pairing cancelled")),
        };
        if result.is_err() {
            let _ = self.exchange("cancel", None).await;
        }
        result
    }
}
pub async fn link(config: &mut Config) -> Result<()> {
    require_interactive(
        "Pairing requires verification on this terminal.",
        "Run `envx auth link` in an interactive terminal.",
    )?;
    let key = config.unlocked_primary_key()?;
    key.key.verify_passphrase(&key.password)?;
    // Reject unsuitable identities before issuing a link, without exposing their contents.
    Bundle::from_key(&key.key)?;
    let mut relay =
        Relay::new(config.sdk_url()?, String::new(), None, Some(&key))?;
    let created: Created = relay.post("v2/auth/pairing/new", json!({})).await?;
    relay.id = Uuid::parse_str(&created.id)?.to_string();
    let mut link = relay.base.clone();
    link.set_fragment(Some(&format!("{PREFIX}{}", relay.id)));
    println!("On your new machine, run:\n\nenvx auth login '{link}'\n\nKeep this terminal open. This invitation expires in 10 minutes.");
    relay.bounded(send(&relay, &key)).await
}
async fn send(relay: &Relay<'_>, key: &UnlockedKey) -> Result<()> {
    let mut handshake =
        Handshake::new(&format!("{}:{}", relay.base, relay.id), false)?;
    handshake.read(&relay.wait(1).await?)?;
    relay
        .exchange("handshake", Some(handshake.write()?))
        .await?;
    handshake.read(&relay.wait(3).await?)?;
    let mut channel = handshake.finish()?;
    println!("A machine requested your identity. Only approve a terminal you control.\nEnter the verification code shown by `envx auth login` on that machine.");
    let entered =
        input("Receiving terminal's verification code", false).await?;
    if entered.trim() != channel.code() {
        bail!("Verification code does not match. No identity was sent.");
    }
    let bundle = serde_json::to_vec(&Bundle::from_key(&key.key)?)?;
    relay
        .exchange("payload", Some(channel.encrypt(&bundle)?))
        .await?;
    println!("Encrypted identity sent to the verified machine. Finish login there using your existing passphrase.");
    Ok(())
}
pub async fn login(config: &mut Config, link: &str) -> Result<()> {
    require_interactive(
        "Login requires an interactive terminal.",
        "Run `envx auth login <link>` in an interactive terminal.",
    )?;
    if config.primary_key.is_some() {
        bail!("An identity already exists; login will not replace it");
    }
    let (base, id) = parse_link(link)?;
    let token = hex::encode(rand::random::<[u8; 32]>());
    let relay = Relay::new(base, id, Some(token), None)?;
    let received = relay.bounded(receive(&relay)).await?;
    // Installation is a local commit, outside the cancellable transfer window.
    if let Err(error) = received.bundle.install(
        config,
        relay.base.as_str(),
        received.key.clone(),
        &received.public,
    ) {
        let _ = relay.exchange("cancel", None).await;
        return Err(error);
    }
    if crate::utils::keyring::set_password(
        &received.key.fingerprint,
        &received.password,
        config.get_settings().get_keyring_expiry(),
    )
    .is_err()
    {
        eprintln!("Identity installed. Your system keyring could not cache its passphrase; envx will ask when needed.");
    }
    println!("Logged in successfully. Your existing projects are available; use `envx link` to connect this directory.");
    if relay.exchange("ack", None).await.is_err() {
        eprintln!("Identity installed; relay cleanup could not be confirmed. The encrypted transfer expires automatically.");
    }
    Ok(())
}
struct Received {
    bundle: Bundle,
    key: crate::utils::key::Key,
    public: String,
    password: String,
}
async fn receive(relay: &Relay<'_>) -> Result<Received> {
    let mut handshake =
        Handshake::new(&format!("{}:{}", relay.base, relay.id), true)?;
    relay.exchange("claim", Some(handshake.write()?)).await?;
    handshake.read(&relay.wait(2).await?)?;
    relay
        .exchange("handshake", Some(handshake.write()?))
        .await?;
    let mut channel = handshake.finish()?;
    println!("Verification code:\n{}\n\nEnter this code in your original machine's waiting terminal.\nWaiting for that machine to approve...",channel.code());
    let ciphertext = relay.wait(4).await?;
    let bundle: Bundle = serde_json::from_slice(&channel.decrypt(&ciphertext)?)
        .context("Invalid identity bundle")?;
    let password = input("Existing identity passphrase", true).await?;
    let (key, public, auth) = bundle.validate(&password)?;
    let response = relay
        .http
        .post(format!(
            "{}/test-auth",
            relay.base.as_str().trim_end_matches('/')
        ))
        .bearer_auth(auth)
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("Transferred identity could not authenticate; nothing was installed");
    }
    Ok(Received {
        bundle,
        key,
        public,
        password,
    })
}

// Poll in bounded blocking tasks so cancellation never leaves a prompt thread
// keeping the process alive. The guard restores the terminal on every exit.
async fn input(label: &str, secret: bool) -> Result<String> {
    use crossterm::event::{self, Event, KeyCode, KeyModifiers};
    use std::io::Write;
    struct RestoreTerminal;
    impl Drop for RestoreTerminal {
        fn drop(&mut self) {
            let _ = crossterm::terminal::disable_raw_mode();
            eprintln!();
        }
    }
    eprint!("{label}: ");
    std::io::stderr().flush()?;
    crossterm::terminal::enable_raw_mode()?;
    let _restore = RestoreTerminal;
    let mut value = String::new();
    loop {
        let event = tokio::task::spawn_blocking(
            || -> std::io::Result<Option<Event>> {
                if event::poll(Duration::from_millis(100))? {
                    Ok(Some(event::read()?))
                } else {
                    Ok(None)
                }
            },
        )
        .await??;
        if let Some(Event::Key(key)) = event {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c' | 'd'))
            {
                bail!("Pairing cancelled");
            }
            match key.code {
                KeyCode::Enter => return Ok(value),
                KeyCode::Esc => bail!("Pairing cancelled"),
                KeyCode::Backspace => {
                    if value.pop().is_some() && !secret {
                        eprint!("\x08 \x08");
                    }
                }
                KeyCode::Char(c) if !c.is_control() && value.len() < 4096 => {
                    value.push(c);
                    if !secret {
                        eprint!("{c}");
                    }
                }
                _ => {}
            }
            std::io::stderr().flush()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invitation_contains_only_session_and_safe_server() {
        let id = Uuid::new_v4();
        let link = format!("https://api.envx.sh/#{PREFIX}{id}");
        let (base, session) = parse_link(&link).unwrap();
        assert_eq!(base.as_str(), "https://api.envx.sh/");
        assert_eq!(session, id.to_string());
        for bad in [
            format!("http://example.com/#{PREFIX}{id}"),
            format!("https://user:pass@example.com/#{PREFIX}{id}"),
            format!("https://api.envx.sh/?secret=x#{PREFIX}{id}"),
            "https://api.envx.sh/#key-material".into(),
        ] {
            assert!(parse_link(&bad).is_err());
        }
    }
}
