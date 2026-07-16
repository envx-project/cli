use super::*;
use crate::{sdk::api_url, utils::config::Config};
use anyhow::bail;
use reqwest::header;

/// Test authentication with the server
#[derive(Parser)]
pub struct Args {
    /// Debug output
    #[arg(short, long)]
    debug: bool,
}

pub async fn command(args: Args, config: &mut Config) -> anyhow::Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;
    let fingerprint = key.fingerprint[..8].to_string();

    if key.uuid.is_none() {
        bail!("Key does not have a UUID, try `envx upload`");
    }

    let key = key.unlock(&password);

    let client = reqwest::Client::new();
    let auth_token = key.auth_token()?;

    if args.debug {
        println!("auth token:\n{}", auth_token.signature);
    }

    let url = format!("{}test-auth", api_url());

    if args.debug {
        dbg!(&url);
    }

    let res = client
        .post(url)
        .header(header::AUTHORIZATION, auth_token.bearer())
        .send()
        .await?;

    if args.debug {
        dbg!(&res);
    }

    let status = res.status();

    if status.is_success() {
        let text = res.text().await?;

        println!("{} fingerprint: {}", "✓ authenticated".green(), fingerprint);
        if args.debug {
            println!("{}", text);
        }
    } else {
        println!("status: {}", status);
        bail!("failed to auth")
    }

    Ok(())
}
