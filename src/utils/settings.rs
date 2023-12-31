use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub algo: Option<String>,
}

impl Settings {
    pub fn default() -> Self {
        Settings { algo: None }
    }
}
