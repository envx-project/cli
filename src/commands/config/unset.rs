use super::fields::{self, Field, FIELDS};
use super::*;
use crate::utils::config::Config;
use crate::utils::prompt::{get_render_config, is_interactive};
use anyhow::bail;

/// Clear an optional config field (interactive when path omitted)
#[derive(Parser)]
pub struct Args {
    /// Field path to unset
    pub path: Option<String>,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let field = match args.path.as_deref() {
        Some(p) => fields::find(p)?,
        None => {
            if !is_interactive() {
                bail!(
                    "No <path> given and stdin is not a terminal.\n\
                     Pass a path: `envx config unset <path>`.",
                );
            }
            pick_unsettable(config)?
        }
    };

    fields::unset(config, field)?;
    println!("{} {}", "unset".green(), field.path);
    Ok(())
}

fn pick_unsettable(config: &Config) -> Result<&'static Field> {
    let candidates: Vec<&'static Field> =
        FIELDS.iter().filter(|f| f.can_unset).collect();
    if candidates.is_empty() {
        bail!("No fields are unset-able.");
    }

    let width = candidates.iter().map(|f| f.path.len()).max().unwrap_or(0);
    let options: Vec<UnsetChoice> = candidates
        .into_iter()
        .map(|f| UnsetChoice {
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

    let picked = inquire::Select::new("Field to unset", options)
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for field")?;
    Ok(picked.field)
}

struct UnsetChoice {
    field: &'static Field,
    label: String,
}

impl std::fmt::Display for UnsetChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}
