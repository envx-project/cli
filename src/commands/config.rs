use crate::utils::{
    btreemap::{FromBTreeMap, ToBTreeMap},
    config::get_config,
    settings::Settings,
};

use super::*;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(short, long)]
    global: bool,

    setting: String,
}

pub async fn command(args: Args, _json: bool) -> Result<()> {
    let mut config = get_config()?;

    let (key, value) = args.setting.split_once("=").unwrap_or_else(|| {
        eprintln!("Error: Invalid setting");
        std::process::exit(1);
    });

    let default = Settings::default().to_btreemap()?;
    let possible_keys = default.keys().collect::<Vec<_>>();
    if !possible_keys.contains(&&key.clone().to_string()) {
        let owned_possible_keys = possible_keys
            .iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>();

        let possible_keys = owned_possible_keys.join(", ");

        Err(anyhow!(
            "Error: Invalid setting.\nPossible values are: {}",
            possible_keys
        ))?
    }

    let mut settings_btreemap = config.get_settings()?.to_btreemap()?;

    if settings_btreemap.contains_key(key) {
        settings_btreemap.remove(key);
    }

    settings_btreemap.insert(key.to_string(), value.to_string());

    let settings = Settings::from_btreemap(&settings_btreemap)?;

    config.settings = Some(settings);
    config.write(true)?;

    Ok(())
}
