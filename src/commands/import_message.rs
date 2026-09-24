use super::*;
use crate::{
    sdk::SDK,
    utils::{
        choice::Choice,
        config::Config,
        kvpair::KVPair,
        messaging::{self, Client},
        messaging_crypto::Payload,
        prompt::prompt_confirm,
    },
};

#[derive(Parser, Debug)]
pub struct Args {
    pub id: String,
    #[arg(short, long)]
    pub project_id: Option<String>,
    /// Apply the names-only preview without interactive confirmation
    #[arg(short, long)]
    pub yes: bool,
    #[arg(long)]
    pub json: bool,
    /// Show names and overwrite conflicts without changing the project
    #[arg(long)]
    pub dry_run: bool,
}
pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let client = Client::new(config)?;
    let envelope = client.read(&args.id).await?;
    let Payload::Variables(variables) = envelope.payload else {
        anyhow::bail!("This message contains text, not named variables");
    };
    if variables.is_empty()
        || variables.keys().any(|name| {
            name.is_empty()
                || !name.bytes().enumerate().all(|(i, b)| {
                    b == b'_'
                        || b.is_ascii_alphabetic()
                        || (i > 0 && b.is_ascii_digit())
                })
        })
    {
        anyhow::bail!("Message contains no variables or an invalid environment variable name");
    }
    let project_id =
        Choice::try_project(config, args.project_id, &client.key).await?;
    let existing = SDK::get_variables(config, &project_id, &client.key).await?;
    let mut replace_ids = Vec::new();
    let mut conflicts = Vec::new();
    for variable in &existing {
        if variables.contains_key(&variable.value.key) {
            replace_ids.push(variable.id.clone());
            conflicts.push(variable.value.key.clone());
        }
    }
    conflicts.sort();
    conflicts.dedup();
    let names = variables.keys().cloned().collect::<Vec<_>>();
    let preview = serde_json::json!({"project_id":project_id,"names":names,"conflicts":conflicts,"applied":false});
    if args.dry_run {
        if args.json {
            println!("{preview}");
        } else {
            eprintln!("{}", serde_json::to_string_pretty(&preview)?);
        }
        return Ok(());
    }
    if !args.json {
        eprintln!(
            "Import to {}: {}",
            project_id,
            names
                .iter()
                .map(|s| messaging::safe(s))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if !conflicts.is_empty() {
            eprintln!(
                "Overwrite: {}",
                conflicts
                    .iter()
                    .map(|s| messaging::safe(s))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    if !args.yes && !prompt_confirm("Apply this import atomically?")? {
        return Ok(());
    }
    let values = variables
        .into_iter()
        .map(|(key, value)| KVPair::new(key, value))
        .collect();
    let ids = SDK::replace_many(
        config,
        values,
        &project_id,
        &client.key,
        replace_ids,
    )
    .await?;
    if let Err(error) =
        crate::utils::cache::wipe_cache_for_project(config, &project_id)
    {
        eprintln!("Import succeeded, but invalidating the local cache failed: {error}");
    }
    if args.json {
        println!(
            "{}",
            serde_json::json!({"project_id":project_id,"names":names,"conflicts":conflicts,"applied":true,"ids":ids})
        );
    } else {
        println!("Imported {} variables.", ids.len());
    }
    Ok(())
}
