use std::process::Stdio;

use super::*;
use tokio::{io::AsyncWriteExt, process::Command};

/// Decrypt a string using GPG
#[derive(Parser)]
pub struct Args {
    /// string to encrypt
    text: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let mut gpg = Command::new("gpg")
        .arg("--yes")
        .arg("--quiet")
        .arg("-d")
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
        println!("\n{}", String::from_utf8(output.stdout)?);
    } else {
        eprintln!("{}", String::from_utf8(output.stderr)?);
    }
    Ok(())
}
