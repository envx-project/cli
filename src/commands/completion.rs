use crate::utils::config::Config;

use super::*;

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

/// Generate completion script
#[derive(Parser)]
pub struct Args {
    shell: Shell,
}

pub async fn command(args: Args, _config: &mut Config) -> Result<()> {
    generate(
        args.shell,
        &mut crate::Args::command(),
        "envx",
        &mut io::stdout(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bash_and_zsh_completion_include_root_commands() {
        for shell in [Shell::Bash, Shell::Zsh] {
            let mut output = Vec::new();
            generate(shell, &mut crate::Args::command(), "envx", &mut output);
            let output = String::from_utf8(output).unwrap();
            for command in
                ["friend-link", "add-friend", "inbox", "project", "variables"]
            {
                assert!(
                    output.contains(command),
                    "{shell:?} missing {command}"
                );
            }
        }
    }
}
