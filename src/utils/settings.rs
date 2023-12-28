use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub test: String,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            test: "test".to_string(),
        }
    }
}
