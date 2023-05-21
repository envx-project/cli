use crate::utils::table::Table;

use super::*;
use anyhow::Ok;
use std::collections::BTreeMap;

/// Print all variables as key=value pairs
#[derive(Parser)]
pub struct Args {
    /// Pretty print as table
    #[clap(short, long)]
    table: bool,
}

pub async fn command(args: Args, json: bool) -> Result<()> {
    let variables = crate::sdk::Client::get_variables().await.unwrap();

    if args.table {
        {
            let map = BTreeMap::from_iter(
                variables
                    .iter()
                    .map(|env| (env.name.clone(), env.value.clone())),
            );
            Table::new("Variables".to_string(), map).print()?;
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&variables)?);
        return Ok(());
    } else {
        for variable in variables {
            println!("{}", variable);
        }
    }

    Ok(())
}
