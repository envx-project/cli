pub(super) use crate::consts::*;
pub(super) use anyhow::Result;
pub(super) use clap::Parser;
pub(super) use colored::Colorize;
pub(super) use std::result::Result::Ok as Good;

pub mod decrypt;
pub mod encrypt;
pub mod gen;
pub mod genkey;
pub mod login;
pub mod rpgp;
pub mod run;
pub mod set;
pub mod shell;
pub mod unset;
pub mod variables;
