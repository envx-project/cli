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
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            warn_on_short_passwords: false,
            keyring_expiry: Some(KeyringExpiry::Days(30)),
        }
    }
}

impl Settings {
    pub fn set_keyring_expiry(&mut self, days: u32) {
        self.keyring_expiry = Some(KeyringExpiry::Days(days));
    }
    pub fn set_keyring_expiry_never(&mut self) {
        self.keyring_expiry = Some(KeyringExpiry::Never);
    }
    pub fn get_keyring_expiry_days(&self) -> Option<u32> {
        match self.keyring_expiry {
            Some(KeyringExpiry::Days(d)) => Some(d),
            _ => None,
        }
    }

    pub fn get_keyring_expiry(&self) -> KeyringExpiry {
        let expiry = self.keyring_expiry.clone();
        expiry.unwrap_or(KeyringExpiry::Days(30))
    }
}
