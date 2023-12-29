use super::*;

#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args, _json: bool) -> Result<()> {
    println!("{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
