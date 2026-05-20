use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum KeyringExpiry {
    Never,
    Days(u32),
}

pub const DEFAULT_MAX_VARIABLES_PER_PROJECT: u32 = 256;
pub const DEFAULT_MAX_PROJECT_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub warn_on_short_passwords: bool,
    pub keyring_expiry: Option<KeyringExpiry>,
    pub loud: Option<bool>,
    pub max_variables_per_project: Option<u32>,
    pub max_project_bytes: Option<u64>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            warn_on_short_passwords: false,
            keyring_expiry: Some(KeyringExpiry::Days(30)),
            loud: None,
            max_variables_per_project: None,
            max_project_bytes: None,
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

    pub fn get_max_variables_per_project(&self) -> u32 {
        self.max_variables_per_project
            .unwrap_or(DEFAULT_MAX_VARIABLES_PER_PROJECT)
    }

    pub fn get_max_project_bytes(&self) -> u64 {
        self.max_project_bytes.unwrap_or(DEFAULT_MAX_PROJECT_BYTES)
    }
}
