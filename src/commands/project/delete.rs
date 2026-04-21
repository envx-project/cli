use anyhow::bail;

use crate::{
    sdk::SDK,
    utils::{
        choice::Choice,
        config::Config,
        prompt::{is_interactive, prompt_confirm_with_default},
    },
};

use super::*;

#[derive(Parser)]
pub struct Args {
    /// Project ID that you want do delete
    project: Option<String>,

    /// Skip the confirmation prompt
    #[arg(short, long)]
    yes: bool,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let password = config.primary_key_password()?;

    let key = key.unlock(&password);

    let project_id = Choice::try_project(args.project, &key).await?;

    if !args.yes {
        if !is_interactive() {
            bail!(
                "{}\n{}",
                "Refusing to delete project without confirmation in a non-interactive terminal.".red(),
                "Re-run with --yes (-y) to confirm deletion.",
            );
        }

        let confirmed = prompt_confirm_with_default(
            &format!("Delete project {}? This cannot be undone.", &project_id),
            false,
        )?;

        if !confirmed {
            println!("Aborting...");
            return Ok(());
        }
    }

    SDK::delete_project(&key, &project_id).await?;

    config.delete_project(&project_id)?;
    println!("Project {} deleted", &project_id);
    Ok(())
}
