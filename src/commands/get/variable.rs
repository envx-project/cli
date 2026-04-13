use std::io::{IsTerminal, Write};

use anyhow::bail;

use crate::utils::{choice::Choice, magic_variables::get_variables_magic};

use super::*;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    project_id: Option<String>,

    #[clap(short, long)]
    key: String,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);
    let project_id = Choice::try_project(args.project_id, &key).await?;

    let kvpairs = get_variables_magic(&project_id, &key, false).await?;

    let kvpair = kvpairs.iter().find(|&kv| kv.key == args.key);

    match kvpair {
        Some(kvpair) => {
            let mut stdout = std::io::stdout();
            stdout.write_all(kvpair.key.as_bytes())?;
            if stdout.is_terminal() {
                stdout.write_all(b"\n")?;
            }
        }
        None => {
            bail!("Key not found");
        }
    }

    Ok(())
}
