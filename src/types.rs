use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub created_at: String, // DateTime
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialUser {
    pub id: String,
    pub username: String,
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

impl From<User> for PartialUser {
    fn from(user: User) -> Self {
        PartialUser {
            id: user.id,
            username: user.username,
        }
    }
}

impl Display for PartialUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.username, self.id)
    }
}
