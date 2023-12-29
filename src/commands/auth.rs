use super::*;
use crate::utils::auth::get_token;
use anyhow::bail;
use reqwest::header;

#[derive(Parser)]
pub struct Args {
    /// Key to sign with
    #[clap(short, long)]
    key: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let client = reqwest::Client::new();

    let auth_token = get_token(&args.key, "dd2bc69b-cabe-4e35-903e-ef76485bd757")
        .await
        .context("Failed to get token")?;

    let res = client
        .post("http://localhost:3000/test-auth")
        .header(header::AUTHORIZATION, format!("Bearer {}", auth_token))
        .send()
        .await?;

    let status = res.status();

    if status.is_success() {
        println!("success");
        // print the text response

        let text = res.text().await?;

        println!("{}", text);
    } else {
        println!("status: {}", status);
        bail!("failed to auth")
    }

    Ok(())
}
