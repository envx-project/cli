use super::*;
use crate::utils::{
    btreemap::ToBTreeMap, choice::Choice, config::Config,
    magic_variables::get_variables_magic, table::Table,
};
/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    project_id: Option<String>,

    /// Output as JSON - JSON has the highest precedence and will override other output formats
    #[clap(long)]
    json: bool,

    /// Output as a list of key=value pairs
    #[clap(long)]
    kv: bool,

    /// Output all variables (this project only)
    #[clap(short, long, default_value_t = false)]
    all: bool,
}

pub async fn command(args: Args, config: Config) -> Result<()> {
    let mode = Mode::from_args(&args);

    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);
    let project_id = Choice::try_project(args.project_id, &key).await?;

    let kvpairs = get_variables_magic(&project_id, &key, args.all).await?;

    match mode {
        Mode::KV => {
            kvpairs.iter().for_each(|kv| println!("{}", kv));
        }
        Mode::Json => {
            let btreemap = kvpairs.to_btreemap()?;
            println!("{}", serde_json::to_string_pretty(&btreemap)?);
        }
        Mode::Table => {
            let btreemap = kvpairs.to_btreemap()?;
            Table::new("Variables".into(), btreemap).print()?;
        }
    }

    Ok(())
}

enum Mode {
    KV,
    Json,
    Table,
}

impl Mode {
    fn from_args(args: &Args) -> Self {
        if args.json {
            Self::Json
        } else if args.kv {
            Self::KV
        } else {
            Self::Table
        }
    }
}
