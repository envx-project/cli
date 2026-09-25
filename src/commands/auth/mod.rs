use super::*;
mod channel;
mod identity;
mod pairing;
mod status;
#[cfg(test)]
mod tests;

/// Manage your identity and securely pair another machine
#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Show diagnostics for the legacy authentication check
    #[arg(long)]
    verbose: bool,
    #[arg(short, long)]
    debug: bool,
}
#[derive(clap::Subcommand)]
enum Commands {
    /// Verify authentication with the server
    Status(status::Args),
    /// Generate a new identity
    Gen(super::gen::Args),
    /// Register an existing local key
    Register(super::upload::Args),
    /// Export a public or secret identity key
    Export(super::export::Args),
    /// Create a pairing invitation and approve the receiving machine
    Link,
    /// Receive your existing identity from another machine
    Login { link: String },
}
pub async fn command(
    args: Args,
    config: &mut crate::utils::config::Config,
) -> anyhow::Result<()> {
    match args.command {
        Some(Commands::Link) => pairing::link(config).await,
        Some(Commands::Login { link }) => pairing::login(config, &link).await,
        Some(Commands::Gen(args)) => super::gen::command(args, config).await,
        Some(Commands::Register(args)) => {
            super::upload::command(args, config).await
        }
        Some(Commands::Export(args)) => {
            super::export::command(args, config).await
        }
        Some(Commands::Status(args)) => status::command(args, config).await,
        None => {
            status::command(
                status::Args {
                    verbose: args.verbose,
                    debug: args.debug,
                },
                config,
            )
            .await
        }
    }
}
