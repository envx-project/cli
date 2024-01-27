use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub test: Option<String>,
}

impl Settings {
    pub fn default() -> Self {
        Settings {
            test: Some("test".to_string())
        }
    }
}
