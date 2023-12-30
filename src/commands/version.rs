use super::*;

#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    println!("{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
