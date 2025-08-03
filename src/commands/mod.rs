pub(super) use anyhow::{Context, Result};
pub(super) use clap::Parser;
pub(super) use colored::Colorize;

// With subcommands
pub mod config;
pub mod get;
pub mod keyring;
pub mod project;

// No subcommands
pub mod auth;
pub mod export;
pub mod gen;
pub mod import;
pub mod link;
pub mod list_keys;
pub mod list_projects;
pub mod run;
pub mod set;
pub mod shell;
pub mod unlink;
pub mod unset;
pub mod update;
pub mod upload;
pub mod variables;
pub mod version;
pub mod whoami;
