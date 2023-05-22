use std::process::Stdio;

use super::*;
use tokio::{io::AsyncWriteExt, process::Command};

/// Encrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    /// recipient's public key
    recipient: String,

    /// string to encrypt
    text: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let mut gpg = Command::new("gg")
        .arg("--batch")
        .arg("--yes")
        .arg("--encrypt")
        .arg("--armor")
        .arg("--recipient")
        .arg("--quiet")
        .arg(args.recipient)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| {
            eprintln!("{}", GPG_ERROR);
            std::process::exit(1)
        });

    {
        let stdin = gpg.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(args.text.as_bytes()).await?;
    }

    let output = gpg.wait_with_output().await?;

    if output.status.success() {
        println!("{}", String::from_utf8(output.stdout)?);
    } else {
        eprintln!("{}", String::from_utf8(output.stderr)?);
    }
    Ok(())
}
