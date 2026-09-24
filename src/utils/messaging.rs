use super::{
    config::Config, key::UnlockedKey, messaging_crypto as crypto,
    state::StateStore,
};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub username: String,
    pub public_key: String,
    pub fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub user: Identity,
    pub created_at: DateTime<Utc>,
    pub receipts: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub creator_id: String,
    pub target_id: Option<String>,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub redeemed_at: Option<DateTime<Utc>>,
    pub redeemed_by: Option<Identity>,
    pub receipt: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkResult {
    pub link: Link,
    pub creator: Identity,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedLink {
    pub link: Link,
    pub token: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub sender_public_key: String,
    pub recipient_public_key: String,
    pub ciphertext: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub user_id: String,
    pub fingerprint: String,
    pub public_key: String,
    pub alias: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FriendCode {
    pub version: u8,
    pub server: String,
    pub id: String,
    pub creator_id: String,
    pub creator_fingerprint: String,
    pub token: String,
}

pub struct Client {
    pub key: UnlockedKey,
    pub origin: String,
    pub state: StateStore,
    http: reqwest::Client,
}
impl Client {
    pub fn new(config: &Config) -> Result<Self> {
        let key = config.unlocked_primary_key()?;
        key.key
            .uuid
            .as_ref()
            .context("Authenticate your key before using friends")?;
        let origin = normalized_origin(config.sdk_url()?.as_str())?;
        Ok(Self {
            key,
            origin,
            state: StateStore::open(config)?,
            http: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
        })
    }
    pub fn user_id(&self) -> &str {
        self.key
            .key
            .uuid
            .as_deref()
            .expect("checked at construction")
    }
    pub async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T> {
        let mut request = self
            .http
            .request(method, format!("{}{}", self.origin, path))
            .header(
                reqwest::header::AUTHORIZATION,
                self.key.auth_token()?.bearer(),
            );
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .context("Unable to reach envx server")?;
        if !response.status().is_success() {
            bail!("Server rejected request (HTTP {})", response.status());
        }
        response.json().await.context("Invalid server response")
    }
    pub async fn delete(&self, path: &str) -> Result<()> {
        let response = self
            .http
            .delete(format!("{}{}", self.origin, path))
            .header(
                reqwest::header::AUTHORIZATION,
                self.key.auth_token()?.bearer(),
            )
            .send()
            .await?;
        if !response.status().is_success() {
            bail!("Server rejected deletion (HTTP {})", response.status());
        }
        Ok(())
    }
    pub async fn friends(&self) -> Result<Vec<Friend>> {
        let mut all = Vec::new();
        let mut before = None;
        loop {
            let page: Vec<Friend> = self
                .request(
                    Method::GET,
                    &page_path("/v2/friends", before.as_deref(), 100)?,
                    None,
                )
                .await?;
            let last = page.last().map(|f| f.user.id.clone());
            let len = page.len();
            all.extend(page);
            if len < 100 {
                break;
            }
            if last == before {
                bail!("Server repeated pagination cursor");
            }
            before = last;
        }
        Ok(all)
    }
    pub fn check_alias(
        &self,
        user_id: &str,
        alias: Option<&str>,
    ) -> Result<()> {
        if let Some(alias) = alias {
            if alias.trim().is_empty()
                || alias.len() > 100
                || alias.chars().any(char::is_control)
                || uuid::Uuid::parse_str(alias).is_ok()
            {
                bail!("Alias must be 1–100 characters, contain no controls, and not be a UUID");
            }
            for (_, pin) in self.state.list::<Pin>("friend")? {
                if pin.user_id != user_id && pin.alias.as_deref() == Some(alias)
                {
                    bail!("Alias already belongs to another friend");
                }
            }
        }
        Ok(())
    }
    pub fn pin(
        &self,
        identity: &Identity,
        mut alias: Option<String>,
        replace: bool,
    ) -> Result<Pin> {
        validate_identity(identity)?;
        self.state.with_write_lock(|_| {
        self.check_alias(&identity.id, alias.as_deref())?;
        if let Some(old) = self.state.get::<Pin>("friend", &identity.id)? {
            if alias.is_none() {
                alias = old.alias.clone();
            }
            if old.fingerprint != identity.fingerprint && !replace {
                bail!("Friend key changed. Verify the fingerprint and explicitly accept it with envx friends --accept-key UUID --fingerprint FINGERPRINT");
            }
        }
        let pin = Pin {
            user_id: identity.id.clone(),
            fingerprint: identity.fingerprint.clone(),
            public_key: identity.public_key.clone(),
            alias,
        };
        self.state.put("friend", &identity.id, &pin)?;
        // Keep old verification keys even after removal or rotation.
        self.state.put(
            "friend-key",
            &format!("{}:{}", pin.user_id, pin.fingerprint),
            &pin,
        )?;
        Ok(pin)
        })
    }
    pub fn trusted(&self, identity: &Identity) -> Result<Pin> {
        validate_identity(identity)?;
        let pin: Pin = self.state.get("friend", &identity.id)?.context("Friend key is not trusted on this machine. Verify it, then use envx friends --accept-key UUID --fingerprint FINGERPRINT")?;
        if pin.fingerprint != identity.fingerprint {
            bail!("Friend key changed; sending is blocked until explicitly accepted");
        }
        Ok(pin)
    }
    pub fn learn_from_receipt(&self, friend: &Friend) -> Result<()> {
        if self.state.get::<Pin>("friend", &friend.user.id)?.is_some() {
            return Ok(());
        }
        validate_identity(&friend.user)?;
        let own = crypto::fingerprint(&self.key.key.public_key_str()?)?;
        for receipt in &friend.receipts {
            if let Ok(receipt) = crypto::verify::<crypto::Receipt>(
                receipt,
                &friend.user.public_key,
            ) {
                if receipt.version == 1
                    && receipt.creator_id == self.user_id()
                    && receipt.creator_fingerprint == own
                    && receipt.redeemer_id == friend.user.id
                    && receipt.redeemer_fingerprint == friend.user.fingerprint
                {
                    // Only a locally generated link authorizes automatically trusting its redeemer.
                    if self
                        .state
                        .get::<String>("created-link", &receipt.id)?
                        .is_some()
                    {
                        self.pin(&friend.user, None, false)?;
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
    pub async fn resolve(&self, target: &str) -> Result<Friend> {
        let friends = self.friends().await?;
        let mut matches = Vec::new();
        for friend in friends {
            self.learn_from_receipt(&friend)?;
            let pin: Option<Pin> = self.state.get("friend", &friend.user.id)?;
            if friend.user.id == target
                || pin.as_ref().and_then(|p| p.alias.as_deref()) == Some(target)
            {
                matches.push(friend);
            }
        }
        if matches.len() != 1 {
            bail!("Use a unique local alias or a friend's stable UUID");
        }
        Ok(matches.remove(0))
    }
    pub async fn read(&self, id: &str) -> Result<crypto::Envelope> {
        let id = uuid::Uuid::parse_str(id)?.to_string();
        let message: Message = self
            .request(Method::GET, &format!("/v2/messages/{id}"), None)
            .await?;
        if message.id != id
            || (message.sender_id != self.user_id()
                && message.recipient_id != self.user_id())
        {
            bail!("Message identity mismatch");
        }
        let sender_fp = crypto::fingerprint(&message.sender_public_key)?;
        let recipient_fp = crypto::fingerprint(&message.recipient_public_key)?;
        let own = crypto::fingerprint(&self.key.key.public_key_str()?)?;
        let (peer_id, peer_fp) = if message.sender_id == self.user_id() {
            if own != sender_fp {
                bail!("Sender key mismatch");
            }
            (&message.recipient_id, &recipient_fp)
        } else {
            if own != recipient_fp {
                bail!("Recipient key mismatch");
            }
            (&message.sender_id, &sender_fp)
        };
        // Historical pins permit reading retained messages after removing a friend.
        if self
            .state
            .get::<Pin>("friend-key", &format!("{peer_id}:{peer_fp}"))?
            .is_none()
        {
            for friend in self.friends().await? {
                self.learn_from_receipt(&friend)?;
            }
        }
        let _: Pin = self
            .state
            .get("friend-key", &format!("{peer_id}:{peer_fp}"))?
            .context("Message peer key is not locally trusted")?;
        let envelope = crypto::decrypt(
            message
                .ciphertext
                .as_deref()
                .context("Missing ciphertext")?,
            &self.key,
            &message.sender_public_key,
        )?;
        if envelope.server != self.origin
            || envelope.id != message.id
            || envelope.sender_id != message.sender_id
            || envelope.recipient_id != message.recipient_id
            || envelope.sender_fingerprint != sender_fp
            || envelope.recipient_fingerprint != recipient_fp
            || envelope.expires_at != message.expires_at
        {
            bail!("Signed envelope does not match message metadata");
        }
        Ok(envelope)
    }
}
pub fn validate_identity(identity: &Identity) -> Result<()> {
    uuid::Uuid::parse_str(&identity.id)?;
    if crypto::fingerprint(&identity.public_key)? != identity.fingerprint {
        bail!("Identity fingerprint does not match public key");
    }
    Ok(())
}
pub fn normalized_origin(value: &str) -> Result<String> {
    let url = url::Url::parse(value)?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        bail!("Messaging requires a plain HTTP(S) server origin");
    }
    Ok(url.origin().ascii_serialization())
}
pub fn page_path(
    path: &str,
    before: Option<&str>,
    limit: u16,
) -> Result<String> {
    if !(1..=100).contains(&limit) {
        bail!("Limit must be between 1 and 100");
    }
    let mut path = format!("{path}?limit={limit}");
    if let Some(before) = before {
        path.push_str(&format!("&before={}", uuid::Uuid::parse_str(before)?));
    }
    Ok(path)
}
pub fn safe(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_control() {
                c.escape_default().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}
pub fn expires(value: &str) -> Result<DateTime<Utc>> {
    let (number, multiplier) = if let Some(v) = value.strip_suffix('h') {
        (v, 3600)
    } else if let Some(v) = value.strip_suffix('d') {
        (v, 86400)
    } else if let Some(v) = value.strip_suffix('m') {
        (v, 60)
    } else {
        bail!("Use a duration such as 24h, 7d, or 30m");
    };
    let seconds = number
        .parse::<i64>()?
        .checked_mul(multiplier)
        .context("Duration too large")?;
    if seconds <= 0 {
        bail!("Expiry must be in the future");
    }
    let duration =
        chrono::Duration::try_seconds(seconds).context("Duration too large")?;
    let expiry = Utc::now()
        .checked_add_signed(duration)
        .context("Duration too large")?;
    // PostgreSQL timestamps have microsecond precision; sign whole seconds so
    // serialization through the server cannot change the authenticated value.
    DateTime::from_timestamp(expiry.timestamp(), 0)
        .context("Duration too large")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_require_exact_origin_and_cursors_are_not_urls() {
        assert_eq!(
            normalized_origin("https://EXAMPLE.com:443/").unwrap(),
            "https://example.com"
        );
        for value in [
            "file:///tmp/a",
            "https://example.com/api",
            "https://user@example.com",
            "https://example.com/?a=b",
            "https://example.com/#secret",
        ] {
            assert!(normalized_origin(value).is_err(), "{value}");
        }
        assert!(page_path("/v2/messages", Some("https://evil.example"), 50)
            .is_err());
        assert!(page_path("/v2/messages", None, 101).is_err());
    }
    #[test]
    fn expiry_and_terminal_metadata_are_bounded() {
        assert!(expires("0h").is_err());
        assert!(expires("-1h").is_err());
        assert!(expires("9999999999999999999999999d").is_err());
        assert!(expires("24h").unwrap() > Utc::now());
        assert_eq!(safe("hi\x1b[31m\n"), "hi\\u{1b}[31m\\n");
        assert_eq!(safe("Māori"), "Māori");
    }
}
