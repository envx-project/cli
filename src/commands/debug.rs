// use crate::utils::vecu8::ToHex;

use crate::sdk::get_api_url;

use super::*;
// use pgp::{composed::message::Message, types::KeyTrait, Deserializable};

/// Unset the current project
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    let api_url = get_api_url();
    println!("API URL: {}", api_url);

    Ok(())
}
