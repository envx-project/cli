use super::*;
use crate::utils::{
    config::Config, messaging::Client, messaging_crypto::Payload,
};
use std::{
    io::{IsTerminal, Write},
    path::PathBuf,
};

/// Explicitly decrypt and reveal a message after signature and trust verification
#[derive(Parser, Debug)]
pub struct Args {
    pub id: String,
    #[arg(long, conflicts_with = "output")]
    pub json: bool,
    /// Write to a new private file instead of stdout; existing files are never overwritten
    #[arg(long)]
    pub output: Option<PathBuf>,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let client = Client::new(config)?;
    let envelope = client.read(&args.id).await?;
    let bytes = if args.json {
        serde_json::to_vec(&envelope)?
    } else {
        match envelope.payload {
            Payload::Text(text) => text.into_bytes(),
            Payload::Variables(variables) => {
                serde_json::to_vec_pretty(&variables)?
            }
        }
    };
    if let Some(path) = args.output {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)?.write_all(&bytes)?;
    } else {
        if std::io::stdout().is_terminal() && !args.json {
            // Secrets may contain terminal escape sequences; redirected output is exact.
            let text = String::from_utf8(bytes)?;
            let visible: String = text
                .chars()
                .map(|c| {
                    if c.is_control() && c != '\n' && c != '\t' {
                        c.escape_default().to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect();
            std::io::stdout().lock().write_all(visible.as_bytes())?;
        } else {
            std::io::stdout().lock().write_all(&bytes)?;
        }
        if args.json {
            println!();
        }
    }
    Ok(())
}
