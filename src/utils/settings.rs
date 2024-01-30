use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub warn_on_short_passwords: bool,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            warn_on_short_passwords: false,
        }
    }
}
