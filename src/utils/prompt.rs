use std::fmt::Display;

use anyhow::{Context, Result};

use inquire::ui::{Attributes, RenderConfig, StyleSheet, Styled};
pub fn get_render_config() -> RenderConfig {
    RenderConfig::default_colored()
        .with_help_message(
            StyleSheet::new()
                .with_fg(inquire::ui::Color::LightMagenta)
                .with_attr(Attributes::BOLD),
        )
        .with_answer(
            StyleSheet::new()
                .with_fg(inquire::ui::Color::LightCyan)
                .with_attr(Attributes::BOLD),
        )
        .with_prompt_prefix(
            Styled::new("?").with_style_sheet(
                StyleSheet::new()
                    .with_fg(inquire::ui::Color::LightCyan)
                    .with_attr(Attributes::BOLD),
            ),
        )
}

#[allow(dead_code)]
pub fn prompt_options<T: Display>(message: &str, options: Vec<T>) -> Result<T> {
    let select = inquire::Select::new(message, options);
    select
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for options")
}

pub fn prompt_confirm(message: &str) -> Result<bool> {
    let confirm = inquire::Confirm::new(message);
    confirm
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for confirm")
}

#[allow(dead_code)]
pub fn prompt_confirm_with_default(message: &str, default: bool) -> Result<bool> {
    let confirm = inquire::Confirm::new(message);
    confirm
        .with_default(default)
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for confirm")
}

#[allow(dead_code)]
pub fn prompt_multi_options<T: Display>(message: &str, options: Vec<T>) -> Result<Vec<T>> {
    let multi_select = inquire::MultiSelect::new(message, options);
    multi_select
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for multi options")
}

#[allow(dead_code)]
pub fn prompt_text(message: &str) -> Result<String> {
    let text = inquire::Text::new(message);
    text.with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for text")
}

#[allow(dead_code)]
pub fn prompt_select<T: Display>(message: &str, options: Vec<T>) -> Result<T> {
    inquire::Select::new(message, options)
        .with_render_config(get_render_config())
        .prompt()
        .context("Failed to prompt for select")
}
