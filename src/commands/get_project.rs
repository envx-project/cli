use super::*;
use crate::{sdk::SDK, utils::auth::get_token};
use anyhow::bail;
use reqwest::header;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
pub struct Args {
    /// Key to sign with
    #[clap(short, long)]
    key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub created_at: String, // DateTime
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectInfo {
    project_id: String,
    users: Vec<User>,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let project_info = SDK::get_project_info(
        "be38c68e-5003-4493-a7b8-5653e8db26a6",
        &args.key,
        "dd2bc69b-cabe-4e35-903e-ef76485bd757",
    )
    .await?;

    println!("{:?}", project_info);

    Ok(())
}
