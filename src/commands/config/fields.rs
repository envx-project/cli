use crate::utils::config::Config;
use crate::utils::settings::KeyringExpiry;
use anyhow::{anyhow, bail, Result};

pub enum FieldKind {
    Url,
    Bool,
    Password,
    StringList,
    KeyringExpiry,
    Number,
}

pub struct Field {
    pub path: &'static str,
    pub kind: FieldKind,
    pub description: &'static str,
    pub format_hint: &'static str,
    pub can_unset: bool,
}

pub const FIELDS: &[Field] = &[
    Field {
        path: "sdk_url",
        kind: FieldKind::Url,
        description: "Base URL of the envx server",
        format_hint: "URL (e.g. https://api.envx.sh)",
        can_unset: true,
    },
    Field {
        path: "primary_key_password",
        kind: FieldKind::Password,
        description: "Cached password for the primary key (skips keyring)",
        format_hint: "passphrase",
        can_unset: true,
    },
    Field {
        path: "primary_key_command",
        kind: FieldKind::StringList,
        description: "Command to print the primary key password to stdout",
        format_hint: "comma-separated argv (e.g. printf,abc123) or JSON array",
        can_unset: true,
    },
    Field {
        path: "settings.warn_on_short_passwords",
        kind: FieldKind::Bool,
        description: "Warn when a generated key uses a short password",
        format_hint: "true | false",
        can_unset: false,
    },
    Field {
        path: "settings.keyring_expiry",
        kind: FieldKind::KeyringExpiry,
        description: "How long the OS keyring caches the primary password",
        format_hint: "'never' or a positive integer (days)",
        can_unset: true,
    },
    Field {
        path: "settings.loud",
        kind: FieldKind::Bool,
        description: "Print one-line action summaries to stderr (link/gen/run)",
        format_hint: "true | false",
        can_unset: true,
    },
    Field {
        path: "settings.max_variables_per_project",
        kind: FieldKind::Number,
        description: "Reject `envx set` that would push a project over this many variables",
        format_hint: "positive integer (default 256)",
        can_unset: true,
    },
    Field {
        path: "settings.max_project_bytes",
        kind: FieldKind::Number,
        description: "Reject `envx set` that would push a project over this many bytes of plaintext",
        format_hint: "positive integer in bytes (default 104857600 = 100MB)",
        can_unset: true,
    },
];

pub fn find(path: &str) -> Result<&'static Field> {
    if let Some(redirect) = forbidden(path) {
        bail!("{}", redirect);
    }
    FIELDS
        .iter()
        .find(|f| f.path == path)
        .ok_or_else(|| anyhow!(
            "Unknown config path '{}'. Run `envx config get` with no args to see editable fields.",
            path
        ))
}

fn forbidden(path: &str) -> Option<String> {
    let p = path.split('.').next().unwrap_or(path);
    match p {
        "salt" => Some(
            "`salt` is load-bearing and cannot be edited.".to_string(),
        ),
        "primary_key" => Some(
            "Use `envx gen` to create a primary key, or `envx import` to load one.".to_string(),
        ),
        "projects" => Some(
            "Use `envx link`, `envx unlink`, or `envx project delete` instead.".to_string(),
        ),
        _ => None,
    }
}

pub fn current_display(config: &Config, field: &Field) -> String {
    match field.path {
        "sdk_url" => config
            .sdk_url
            .clone()
            .unwrap_or_else(|| "<unset>".to_string()),
        "primary_key_password" => match config.primary_key_password {
            Some(_) => "<set>".to_string(),
            None => "<unset>".to_string(),
        },
        "primary_key_command" => match &config.primary_key_command {
            Some(cmd) => cmd.join(" "),
            None => "<unset>".to_string(),
        },
        "settings.warn_on_short_passwords" => config
            .settings
            .as_ref()
            .map(|s| s.warn_on_short_passwords.to_string())
            .unwrap_or_else(|| "false".to_string()),
        "settings.keyring_expiry" => config
            .settings
            .as_ref()
            .and_then(|s| s.keyring_expiry.clone())
            .map(|e| match e {
                KeyringExpiry::Never => "never".to_string(),
                KeyringExpiry::Days(n) => format!("{} days", n),
            })
            .unwrap_or_else(|| "<unset>".to_string()),
        "settings.loud" => config
            .settings
            .as_ref()
            .and_then(|s| s.loud)
            .map(|b| b.to_string())
            .unwrap_or_else(|| "false".to_string()),
        "settings.max_variables_per_project" => config
            .settings
            .as_ref()
            .and_then(|s| s.max_variables_per_project)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "<unset>".to_string()),
        "settings.max_project_bytes" => config
            .settings
            .as_ref()
            .and_then(|s| s.max_project_bytes)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "<unset>".to_string()),
        _ => "?".to_string(),
    }
}

pub fn current_json(config: &Config, field: &Field) -> serde_json::Value {
    match field.path {
        "sdk_url" => serde_json::to_value(&config.sdk_url).unwrap(),
        "primary_key_password" => {
            serde_json::to_value(&config.primary_key_password).unwrap()
        }
        "primary_key_command" => {
            serde_json::to_value(&config.primary_key_command).unwrap()
        }
        "settings.warn_on_short_passwords" => serde_json::Value::Bool(
            config
                .settings
                .as_ref()
                .map(|s| s.warn_on_short_passwords)
                .unwrap_or(false),
        ),
        "settings.keyring_expiry" => serde_json::to_value(
            config
                .settings
                .as_ref()
                .and_then(|s| s.keyring_expiry.clone()),
        )
        .unwrap(),
        "settings.loud" => {
            serde_json::to_value(config.settings.as_ref().and_then(|s| s.loud))
                .unwrap()
        }
        "settings.max_variables_per_project" => serde_json::to_value(
            config
                .settings
                .as_ref()
                .and_then(|s| s.max_variables_per_project),
        )
        .unwrap(),
        "settings.max_project_bytes" => serde_json::to_value(
            config.settings.as_ref().and_then(|s| s.max_project_bytes),
        )
        .unwrap(),
        _ => serde_json::Value::Null,
    }
}

pub fn set(config: &mut Config, field: &Field, raw: &str) -> Result<()> {
    match field.path {
        "sdk_url" => {
            let parsed = url::Url::parse(raw)
                .map_err(|e| anyhow!("Invalid URL: {}", e))?;
            config.sdk_url =
                Some(parsed.to_string().trim_end_matches('/').to_string());
        }
        "primary_key_password" => {
            if let Some(key) = &config.primary_key {
                key.verify_passphrase(raw)?;
            } else {
                crate::utils::key::validate_passphrase_not_empty(raw)?;
            }
            config.primary_key_password = Some(raw.to_string());
        }
        "primary_key_command" => {
            let parts = parse_string_list(raw)?;
            if parts.is_empty() {
                bail!("primary_key_command needs at least one argument");
            }
            config.primary_key_command = Some(parts);
        }
        "settings.warn_on_short_passwords" => {
            let v = parse_bool(raw)?;
            let mut s = config.settings.clone().unwrap_or_default();
            s.warn_on_short_passwords = v;
            config.settings = Some(s);
        }
        "settings.keyring_expiry" => {
            let v = parse_keyring_expiry(raw)?;
            let mut s = config.settings.clone().unwrap_or_default();
            s.keyring_expiry = Some(v);
            config.settings = Some(s);
        }
        "settings.loud" => {
            let v = parse_bool(raw)?;
            let mut s = config.settings.clone().unwrap_or_default();
            s.loud = Some(v);
            config.settings = Some(s);
        }
        "settings.max_variables_per_project" => {
            let v: u32 = raw.trim().parse().map_err(|_| {
                anyhow!("Expected positive integer, got '{}'", raw.trim())
            })?;
            let mut s = config.settings.clone().unwrap_or_default();
            s.max_variables_per_project = Some(v);
            config.settings = Some(s);
        }
        "settings.max_project_bytes" => {
            let v: u64 = raw.trim().parse().map_err(|_| {
                anyhow!("Expected positive integer, got '{}'", raw.trim())
            })?;
            let mut s = config.settings.clone().unwrap_or_default();
            s.max_project_bytes = Some(v);
            config.settings = Some(s);
        }
        _ => bail!("Field not handled in setter"),
    }
    Ok(())
}

pub fn unset(config: &mut Config, field: &Field) -> Result<()> {
    if !field.can_unset {
        bail!(
            "`{}` cannot be unset. Use `envx config set {} <value>` instead.",
            field.path,
            field.path
        );
    }
    match field.path {
        "sdk_url" => config.sdk_url = None,
        "primary_key_password" => config.primary_key_password = None,
        "primary_key_command" => config.primary_key_command = None,
        "settings.keyring_expiry" => {
            if let Some(s) = config.settings.as_mut() {
                s.keyring_expiry = None;
            }
        }
        "settings.loud" => {
            if let Some(s) = config.settings.as_mut() {
                s.loud = None;
            }
        }
        "settings.max_variables_per_project" => {
            if let Some(s) = config.settings.as_mut() {
                s.max_variables_per_project = None;
            }
        }
        "settings.max_project_bytes" => {
            if let Some(s) = config.settings.as_mut() {
                s.max_project_bytes = None;
            }
        }
        _ => bail!("Field not handled in unsetter"),
    }
    Ok(())
}

fn parse_bool(raw: &str) -> Result<bool> {
    match raw.trim().to_lowercase().as_str() {
        "true" | "t" | "yes" | "y" | "1" => Ok(true),
        "false" | "f" | "no" | "n" | "0" => Ok(false),
        other => bail!("Expected true/false, got '{}'", other),
    }
}

fn parse_keyring_expiry(raw: &str) -> Result<KeyringExpiry> {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("never") {
        return Ok(KeyringExpiry::Never);
    }
    let days: u32 = trimmed.parse().map_err(|_| {
        anyhow!("Expected 'never' or a positive integer, got '{}'", trimmed)
    })?;
    Ok(KeyringExpiry::Days(days))
}

fn parse_string_list(raw: &str) -> Result<Vec<String>> {
    let trimmed = raw.trim();
    if trimmed.starts_with('[') {
        let parsed: Vec<String> = serde_json::from_str(trimmed)
            .map_err(|e| anyhow!("Invalid JSON array: {}", e))?;
        Ok(parsed)
    } else {
        Ok(trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}
