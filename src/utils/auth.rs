use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub signature: String,
}

impl AuthToken {
    pub fn new(token: String, signature: String) -> Self {
        Self { token, signature }
    }

    pub fn bearer(&self) -> String {
        format!("Bearer {}", self)
    }
}

impl From<AuthToken> for String {
    fn from(auth_token: AuthToken) -> String {
        serde_json::to_string(&auth_token).unwrap()
    }
}

impl std::fmt::Display for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string_token = serde_json::to_string(&self).unwrap();

        write!(f, "{}", string_token)
    }
}
