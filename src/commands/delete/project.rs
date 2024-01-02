use crate::{
    sdk::SDK,
    utils::{choice::Choice, config::get_config},
};

use super::*;

#[derive(Parser)]
pub struct Args {
    /// Key fingerprint to use
    #[clap(short, long)]
    key: Option<String>,

    /// Project ID that you want do delete
    project: Option<String>,
}

pub async fn command(args: Args) -> Result<()> {
    let mut config = get_config()?;
    let key = config.get_key_or_default(args.key)?;

    let project_id = Choice::try_project(args.project, &key.fingerprint).await?;
    SDK::delete_project(&project_id, &key.fingerprint).await?;
    config.delete_project(&project_id)?;
    println!("Project {} deleted", &project_id);

    Ok(())
}
