use crate::utils::config::Config;
use colored::Colorize;

pub fn say(config: &Config, msg: impl AsRef<str>) {
    if !config.get_settings().is_loud() {
        return;
    }
    eprintln!("{} {}", "→".green(), msg.as_ref().green());
}
