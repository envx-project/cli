use super::*;
use crate::controllers::variables::get_variables;
use anyhow::Ok;

/// Clean the cache
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    kv: bool,
}

pub async fn command(args: Args, json: bool) -> Result<()> {
    let variables = get_variables().await.unwrap();

    if args.kv {
        if variables.len() == 0 {
            println!("[]");
            return Ok(());
        }

        let formatted_variables = variables
            .iter()
            .map(|env| format!("{}", env))
            .collect::<Vec<String>>()
            .join("\n");

        println!("{}", formatted_variables);
        return Ok(());
    } else if json {
        println!("{}", serde_json::to_string_pretty(&variables)?);
        return Ok(());
    } else {
        unimplemented!();
    }

    #[allow(unreachable_code)]
    Ok(())
}
