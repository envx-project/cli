use serde::{Deserialize, Serialize};
// #[derive(Serialize, Deserialize)]
// pub(crate) struct User {
//     pub id: String,
// }

#[derive(Serialize, Deserialize, Debug)]
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
