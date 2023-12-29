use crate::utils::vecu8::ToHex;

use super::*;
use pgp::{composed::message::Message, types::KeyTrait, Deserializable};

/// Unset the current project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    Ok(())
}
