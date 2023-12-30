use super::*;

#[derive(Parser)]
pub struct Args {}

pub async fn command(_args: Args) -> Result<()> {
    println!(
        "{} {} {} {}",
        "env-cli".cyan(),
        env!("CARGO_PKG_VERSION").magenta(),
        "by".blue(),
        "alexng353".yellow()
    );

    Ok(())
}
