use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum KeyringExpiry {
    Never,
    Days(u32),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub warn_on_short_passwords: bool,
    pub keyring_expiry: Option<KeyringExpiry>,
    pub loud: Option<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            warn_on_short_passwords: false,
            keyring_expiry: Some(KeyringExpiry::Days(30)),
            loud: None,
        }
    }
}

impl Settings {
    pub fn get_keyring_expiry(&self) -> KeyringExpiry {
        let expiry = self.keyring_expiry.clone();
        expiry.unwrap_or(KeyringExpiry::Days(30))
    }

    pub fn is_loud(&self) -> bool {
        self.loud.unwrap_or(false)
    }
}
