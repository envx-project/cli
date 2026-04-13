use std::io::Write;

use anyhow::bail;
use shlex::Shlex;

use crate::utils::prompt::prompt_text;

use super::*;

/// Set the command used to get the primary key password
///
/// This command will be run when the primary key password is requested.
///
/// If no command is provided, the command will be removed.
#[derive(Parser)]
pub struct Args {
    /// Print the command
    #[arg(short, long)]
    print: bool,

    /// Command to run
    command: Option<String>,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    if args.print {
        if let Some(command) = &config.primary_key_command {
            println!("The current command is:\n{}", command.join(" "));
        } else {
            println!("No command is set.");
        }
        return Ok(());
    }

    let command = args.command.unwrap_or_else(|| {
        prompt_text("Command to run").expect("Failed to prompt")
    });
    let command: Vec<String> = Shlex::new(&command).collect();

    println!("The command will be set to:\n{:?}", command);

    println!("Testing the command...");

    if command.is_empty() {
        config.primary_key_command = None;
        println!("No command was provided, so the command was removed.");
        return Ok(());
    }

    let program = command.first().unwrap();
    let output = std::process::Command::new(program)
        .args(command.iter().skip(1))
        .output()
        .expect("Failed to run command");

    if !output.status.success() {
        std::io::stderr().write_all(&output.stderr)?;
        bail!("Command failed. The command was not set. Please try again.");
    }

    let password = String::from_utf8(output.stdout)?;

    println!(
        "\nThe provided command returned:\n\t{}...\n",
        &password[0..4.min(password.len())]
    );

    println!("The command has been set.");
    println!("To remove or edit the command, run `envx config set password_command`.");

    Ok(())
}
