use super::fields::{self, Field, FIELDS};
use super::*;
use crate::utils::config::Config;
use crate::utils::prompt::{get_render_config, is_interactive};

/// Read a config field (lists all when path omitted)
#[derive(Parser)]
pub struct Args {
    /// Field path
    pub path: Option<String>,

    /// Emit JSON instead of a plain string
    #[arg(long)]
    json: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let field = match args.path.as_deref() {
        Some(p) => fields::find(p)?,
        None => {
            if !is_interactive() {
                if args.json {
                    let map: serde_json::Map<String, serde_json::Value> =
                        FIELDS
                            .iter()
                            .map(|f| {
                                (
                                    f.path.to_string(),
                                    fields::current_json(config, f),
                                )
                            })
                            .collect();
                    println!("{}", serde_json::to_string_pretty(&map)?);
                    return Ok(());
                }
                print_table(config);
                return Ok(());
            }
            pick_field(config)?
        }
    };

    if args.json {
        let v = fields::current_json(config, field);
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        println!("{}", fields::current_display(config, field));
    }
    Ok(())
}

fn print_table(config: &Config) {
    let width = FIELDS.iter().map(|f| f.path.len()).max().unwrap_or(0);
    for f in FIELDS {
        println!(
            "{:width$}  {}",
            f.path,
            fields::current_display(config, f),
            width = width
        );
    }
}

fn pick_field(config: &Config) -> Result<&'static Field> {
    let width = FIELDS.iter().map(|f| f.path.len()).max().unwrap_or(0);
    let options: Vec<GetChoice> = FIELDS
        .iter()
        .map(|f| GetChoice {
            field: f,
            label: format!(
                "{:width$}  [{}]  {}",
                f.path,
                fields::current_display(config, f),
                f.description,
                width = width
            ),
        })
        .collect();

    let picked = inquire::Select::new("Field to read", options)
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for field")?;
    Ok(picked.field)
}

struct GetChoice {
    field: &'static Field,
    label: String,
}

impl std::fmt::Display for GetChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}
