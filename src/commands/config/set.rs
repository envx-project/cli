use super::fields::{self, Field, FieldKind, FIELDS};
use super::*;
use crate::utils::config::Config;
use crate::utils::prompt::{
    get_render_config, is_interactive, prompt_confirm_with_default,
    prompt_password, prompt_text,
};
use anyhow::bail;

/// Set a config field (interactive when args omitted)
#[derive(Parser)]
pub struct Args {
    /// Field path (e.g. sdk_url, settings.keyring_expiry)
    pub path: Option<String>,

    /// New value
    pub value: Option<String>,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let field = match args.path.as_deref() {
        Some(p) => fields::find(p)?,
        None => {
            if !is_interactive() {
                bail!(
                    "No <path> given and stdin is not a terminal.\n\
                     Pass a path: `envx config set <path> <value>`. \
                     Run `envx config get` to see fields.",
                );
            }
            pick_field(config)?
        }
    };

    let value = match args.value {
        Some(v) => v,
        None => {
            if !is_interactive() {
                bail!(
                    "No <value> given and stdin is not a terminal.\n\
                     Pass a value: `envx config set {} <value>`.",
                    field.path,
                );
            }
            read_value(field, config)?
        }
    };

    fields::set(config, field, &value)?;
    println!(
        "{} {} = {}",
        "set".green(),
        field.path,
        fields::current_display(config, field),
    );
    Ok(())
}

fn pick_field(config: &Config) -> Result<&'static Field> {
    let width = FIELDS.iter().map(|f| f.path.len()).max().unwrap_or(0);
    let options: Vec<FieldChoice> = FIELDS
        .iter()
        .map(|f| FieldChoice {
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

    let picked = inquire::Select::new("Field to set", options)
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for field")?;
    Ok(picked.field)
}

struct FieldChoice {
    field: &'static Field,
    label: String,
}

impl std::fmt::Display for FieldChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

fn read_value(field: &Field, config: &Config) -> Result<String> {
    let prompt =
        format!("New value for {} ({})", field.path, field.format_hint);
    match field.kind {
        FieldKind::Password => prompt_password(&prompt),
        FieldKind::Bool => {
            let current = matches!(
                fields::current_display(config, field).as_str(),
                "true"
            );
            let v = prompt_confirm_with_default(&prompt, current)?;
            Ok(v.to_string())
        }
        FieldKind::Url
        | FieldKind::StringList
        | FieldKind::KeyringExpiry
        | FieldKind::Number => prompt_text(&prompt),
    }
}
