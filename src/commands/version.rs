use super::*;

/// Fancy, pretty-printed version information.
#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    println!(
        "{} {} {} {}",
        "envx".cyan(),
        env!("CARGO_PKG_VERSION").magenta(),
        "by".blue(),
        "alexng353".yellow()
    );

    Ok(())
}
