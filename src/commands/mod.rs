pub(super) use anyhow::Result;
pub(super) use clap::Parser;
pub(super) use colored::Colorize;
pub(super) use std::result::Result::Ok as Good;

pub mod decrypt;
pub mod delete_key;
pub mod encrypt;
pub mod gen;
pub mod get_config;
pub mod init;
pub mod read_local;
pub mod run;
pub mod set;
pub mod set_local;
pub mod settings;
pub mod shell;
pub mod unset;
pub mod variables;
