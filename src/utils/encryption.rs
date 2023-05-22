use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

pub(crate) async fn encrypt(
    recipient: String,
    text: String,
) -> anyhow::Result<String, anyhow::Error> {
    let mut gpg = Command::new("gpg")
        .arg("--batch")
        .arg("--yes")
        .arg("--encrypt")
        .arg("--armor")
        .arg("--recipient")
        .arg("--quiet")
        .arg(recipient)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| {
            eprintln!("{}", crate::constants::GPG_ERROR);
            std::process::exit(1)
        });

    {
        let stdin = gpg.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(text.as_bytes()).await?;
    }

    let output = gpg.wait_with_output().await?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(anyhow::Error::msg(String::from_utf8(output.stderr)?))
    }
}

pub(crate) async fn decrypt(text: String) -> anyhow::Result<String, anyhow::Error> {
    let mut gpg = Command::new("gpg")
        .arg("--yes")
        .arg("--quiet")
        .arg("-d")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| {
            eprintln!("{}", crate::constants::GPG_ERROR);
            std::process::exit(1)
        });

    {
        let stdin = gpg.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(text.as_bytes()).await?;
    }

    let output = gpg.wait_with_output().await?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(anyhow::Error::msg(String::from_utf8(output.stderr)?))
    }
}
