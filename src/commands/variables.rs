use super::*;
use crate::utils::{
    btreemap::ToBTreeMap, choice::Choice, config::Config,
    magic_variables::get_variables_magic, table::Table,
};
use regex::Regex;
/// Get all environment variables for the current configured directory
#[derive(Parser)]
pub struct Args {
    #[arg(short, long)]
    project_id: Option<String>,

    /// Filter variables by regex syntax
    #[arg(short, long)]
    filter: Option<String>,

    /// Output as JSON - JSON has the highest precedence and will override other output formats
    #[arg(long)]
    json: bool,

    /// Output as a list of key=value pairs
    #[arg(long)]
    kv: bool,

    /// Output all variables (this project only)
    #[arg(short, long, default_value_t = false)]
    all: bool,

    /// Use locally cached variables instead of fetching from the server
    #[arg(long)]
    local: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let mode = Mode::from_args(&args);

    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);
    let project_id = Choice::try_project(args.project_id, &key).await?;

    let mut kvpairs =
        get_variables_magic(&project_id, &key, args.all, args.local).await?;
    if let Some(filter) = &args.filter {
        let re = Regex::new(filter)?;
        kvpairs.retain(|kv| re.is_match(&kv.key));
        if !matches!(mode, Mode::Json) {
            for kv in &mut kvpairs {
                kv.key = re
                    .replace_all(&kv.key, |caps: &regex::Captures| {
                        format!("{}", caps[0].red().bold())
                    })
                    .to_string();
            }
        }
    }

    match mode {
        Mode::KV => {
            kvpairs.sort_by(|a, b| a.key.cmp(&b.key));
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
