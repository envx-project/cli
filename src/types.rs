use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub created_at: String, // DateTime
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectInfo {
    pub project_id: String,
    pub users: Vec<User>,
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.username, self.id)
    }
}
