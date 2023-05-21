use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub(crate) struct User {
    pub id: String,
}
