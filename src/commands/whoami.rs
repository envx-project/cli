use super::*;
use crate::utils::config::Config;

/// Print the primary key fingerprint and uuid
#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    json: bool,
}

pub async fn command(args: Args) -> Result<()> {
    let config = Config::get().await;
    let primary_key = config.primary_key()?;
    if args.json {
        println!("{}", serde_json::to_string(&primary_key)?);
        return Ok(());
    } else {
        println!("fingerprint: {}", &primary_key.fingerprint[..8]);
        println!(
            "uuid: {}",
            primary_key.uuid.unwrap_or("Not on remote".into())
        );
    }
    Ok(())
}
