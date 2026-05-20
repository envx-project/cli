use crate::utils::choice::Choice;
use crate::utils::config::Config;
use crate::utils::env_override::apply_env_overrides;
use crate::utils::magic_variables::get_variables_magic;

use super::*;
use std::collections::BTreeMap;
use std::vec;

/// winapi is only used on windows
#[cfg(target_os = "windows")]
extern crate winapi;
use anyhow::Context;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
#[cfg(target_os = "windows")]
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
    TH32CS_SNAPPROCESS,
};

/// memory management helpers are also only used on windows
#[cfg(target_os = "windows")]
use std::ffi::CStr;
#[cfg(target_os = "windows")]
use std::mem::zeroed;

/// Open a subshell with envx variables available
#[derive(Parser)]
pub struct Args {
    /// Project ID
    #[arg(short, long)]
    project_id: Option<String>,

    #[arg(short, long)]
    silent: bool,

    /// Override or add environment variables (KEY=VALUE), repeatable
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    env_override: Vec<String>,
}

pub async fn command(args: Args, config: &mut Config) -> Result<()> {
    let key = config.primary_key()?;
    let key = key.unlock(&config.primary_key_password()?);

    let project_id = Choice::try_project(args.project_id, &key).await?;

    if project_id.is_empty() {
        return Err(anyhow::anyhow!("No project ID provided"));
    }

    let mut all_variables = BTreeMap::<String, String>::new();
    let project_label = format!("envx:{}", short_project_id(&project_id));

    all_variables.insert("IN_ENVX_SHELL".to_owned(), "true".to_owned());
    all_variables.insert("ENVX_PROJECT_ID".to_owned(), project_id.clone());
    all_variables.insert("ENVX_PROJECT_LABEL".to_owned(), project_label);

    let variables = get_variables_magic(&project_id, &key, false).await?;

    for variable in variables {
        all_variables.insert(variable.key, variable.value);
    }

    apply_env_overrides(&mut all_variables, args.env_override)?;

    let shell = std::env::var("SHELL").unwrap_or(match std::env::consts::OS {
        "windows" => match windows_shell_detection().await {
            Some(WindowsShell::Powershell) => "powershell".to_string(),
            Some(WindowsShell::Cmd) => "cmd".to_string(),
            Some(WindowsShell::Powershell7) => "pwsh".to_string(),
            Some(WindowsShell::NuShell) => "nu".to_string(),
            Some(WindowsShell::ElvSh) => "elvish".to_string(),
            None => "cmd".to_string(),
        },
        _ => "sh".to_string(),
    });

    let shell_options = match shell.as_str() {
        "powershell" => vec!["/nologo"],
        "pwsh" => vec!["/nologo"],
        "cmd" => vec!["/k"],
        _ => vec![],
    };
    let shell_options = shell_options.into_iter().map(str::to_owned).collect();
    let shell_setup = setup_shell_prompt(&shell, shell_options, &project_id)?;

    if !args.silent {
        println!("Entering subshell with envx variables available. Type 'exit' to exit.\n");
    }

    // a bit janky :/
    ctrlc::set_handler(move || {
        // do nothing, we just want to ignore CTRL+C
        // this is for `rails c` and similar REPLs
    })?;

    tokio::process::Command::new(shell)
        .args(shell_setup.args)
        .envs(all_variables)
        .envs(shell_setup.env)
        .spawn()
        .context("Failed to spawn command")?
        .wait()
        .await
        .context("Failed to wait for command")?;

    drop(shell_setup.cleanup);

    println!("Exited subshell, envx variables no longer available.");
    Ok(())
}

struct ShellSetup {
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cleanup: Option<ShellTempDir>,
}

struct ShellTempDir {
    path: PathBuf,
}

impl Drop for ShellTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn short_project_id(project_id: &str) -> String {
    project_id.chars().take(8).collect()
}

fn setup_shell_prompt(
    shell: &str,
    mut args: Vec<String>,
    project_id: &str,
) -> Result<ShellSetup> {
    let short_id = short_project_id(project_id);
    let mut env = BTreeMap::from([("ENVX_SHORT".to_owned(), short_id)]);
    let mut cleanup = None;

    if shell.ends_with("/bash") || shell == "bash" {
        let temp_dir = create_shell_temp_dir()?;
        let rcfile = temp_dir.path.join("bashrc");
        std::fs::write(
            &rcfile,
            r#"[ -f "$HOME/.bashrc" ] && . "$HOME/.bashrc"
export PS1="(envx:${ENVX_SHORT}) $PS1"
"#,
        )
        .context("Failed to write envx bash rcfile")?;
        args.push("--rcfile".to_owned());
        args.push(rcfile.to_string_lossy().into_owned());
        cleanup = Some(temp_dir);
    } else if shell.ends_with("/zsh") || shell == "zsh" {
        let temp_dir = create_shell_temp_dir()?;
        let zshrc = temp_dir.path.join(".zshrc");
        std::fs::write(
            &zshrc,
            r#"[ -f "${ENVX_ORIG_ZDOTDIR:-$HOME}/.zshrc" ] && . "${ENVX_ORIG_ZDOTDIR:-$HOME}/.zshrc"
PROMPT="(envx:${ENVX_SHORT}) $PROMPT"
"#,
        )
        .context("Failed to write envx zshrc")?;
        let orig_zdotdir = std::env::var("ZDOTDIR")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_default();
        env.insert("ENVX_ORIG_ZDOTDIR".to_owned(), orig_zdotdir);
        env.insert(
            "ZDOTDIR".to_owned(),
            temp_dir.path.to_string_lossy().into_owned(),
        );
        cleanup = Some(temp_dir);
    } else if shell.ends_with("/fish") || shell == "fish" {
        args.push("--init-command".to_owned());
        args.push(
            "functions -c fish_prompt _envx_old_fish_prompt; function fish_prompt; printf \"(envx:%s) \" $ENVX_SHORT; _envx_old_fish_prompt; end"
                .to_owned(),
        );
    }

    Ok(ShellSetup { args, env, cleanup })
}

fn create_shell_temp_dir() -> Result<ShellTempDir> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System clock is before Unix epoch")?
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("envx-shell-{}-{nanos}", std::process::id()));
    std::fs::create_dir(&path)
        .context("Failed to create envx shell tempdir")?;
    Ok(ShellTempDir { path })
}

#[cfg(target_os = "windows")]
unsafe fn node_fix_recursive(
    process_id: DWORD,
    recursion: Option<u32>,
) -> Result<(u32, String)> {
    // recursive because for some reason it occasionally is more than one level deep
    let recursion = recursion.unwrap_or(0);
    if recursion > 10 {
        // no error, just return nothing and it will default to cmd.
        return Ok((0, "".to_string()));
    }

    let (ppid, ppname) = unsafe {
        get_parent_process_info(Some(process_id))
            .context("Failed to get parent process info")
            .unwrap_or_else(|_| (0, "".to_string()))
    };

    if ppname == "node.exe" {
        node_fix_recursive(ppid, recursion.checked_add(1))
    } else {
        Ok((ppid, ppname))
    }
}

// disable dead code warning for windows_shell_detection
#[allow(dead_code)]
enum WindowsShell {
    Cmd,
    Powershell,
    Powershell7,
    NuShell,
    ElvSh,
}

/// https://gist.github.com/mattn/253013/d47b90159cf8ffa4d92448614b748aa1d235ebe4
///
/// defaults to cmd if no parent process is found
#[cfg(target_os = "windows")]
async fn windows_shell_detection() -> Option<WindowsShell> {
    let (ppid, mut ppname) = unsafe {
        get_parent_process_info(None)
            .context("Failed to get parent process info")
            .unwrap_or_else(|_| (0, "".to_string()))
    };

    if ppname == "node.exe" {
        (_, ppname) = unsafe {
            node_fix_recursive(ppid, None)
                .context("Failed to get parent process info")
                // acceptable return because if it fails it will default to cmd
                .unwrap_or_else(|_| (0, "".to_string()))
        }
    }

    let ppname = ppname.split(".").next().unwrap_or("cmd");

    match ppname {
        "cmd" => Some(WindowsShell::Cmd),
        "powershell" => Some(WindowsShell::Powershell),
        "pwsh" => Some(WindowsShell::Powershell7),
        "nu" => Some(WindowsShell::NuShell),
        "elvish" => Some(WindowsShell::ElvSh),
        _ => Some(WindowsShell::Cmd),
    }
}

#[cfg(not(target_os = "windows"))]
async fn windows_shell_detection() -> Option<WindowsShell> {
    None
}

/// get the parent process info, translated from
// https://gist.github.com/mattn/253013/d47b90159cf8ffa4d92448614b748aa1d235ebe4
#[cfg(target_os = "windows")]
unsafe fn get_parent_process_info(
    pid: Option<DWORD>,
) -> Option<(DWORD, String)> {
    let pid = pid.unwrap_or(std::process::id());

    let mut pe32: PROCESSENTRY32 = unsafe { zeroed() };
    let h_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    let mut ppid = 0;

    if h_snapshot == INVALID_HANDLE_VALUE {
        return None;
    }

    pe32.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    if unsafe { Process32First(h_snapshot, &mut pe32) } != 0 {
        loop {
            if pe32.th32ProcessID == pid {
                ppid = pe32.th32ParentProcessID;
                break;
            }
            if unsafe { Process32Next(h_snapshot, &mut pe32) } == 0 {
                break;
            }
        }
    }

    let mut parent_process_name = None;
    if ppid != 0 {
        parent_process_name = get_process_name(ppid);
    }

    unsafe { CloseHandle(h_snapshot) };

    if let Some(ppname) = parent_process_name {
        Some((ppid, ppname))
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_process_name(pid: DWORD) -> Option<String> {
    let mut pe32: PROCESSENTRY32 = unsafe { zeroed() };
    let h_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

    if h_snapshot == INVALID_HANDLE_VALUE {
        return None;
    }

    pe32.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    if unsafe { Process32First(h_snapshot, &mut pe32) } != 0 {
        loop {
            if pe32.th32ProcessID == pid {
                let process_name_cstr =
                    unsafe { CStr::from_ptr(pe32.szExeFile.as_ptr()) };
                let process_name =
                    process_name_cstr.to_string_lossy().into_owned();
                unsafe { CloseHandle(h_snapshot) };
                return Some(process_name);
            }
            if unsafe { Process32Next(h_snapshot, &mut pe32) } == 0 {
                break;
            }
        }
    }

    unsafe { CloseHandle(h_snapshot) };
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT_ID: &str = "12345678-90ab-cdef-1234-567890abcdef";

    #[test]
    fn bash_setup_uses_rcfile_with_prompt_prefix() {
        let setup =
            setup_shell_prompt("/bin/bash", Vec::new(), PROJECT_ID).unwrap();

        assert_eq!(setup.env.get("ENVX_SHORT"), Some(&"12345678".to_owned()));
        assert_eq!(setup.args[0], "--rcfile");
        let rcfile = PathBuf::from(&setup.args[1]);
        let contents = std::fs::read_to_string(&rcfile).unwrap();
        assert!(
            contents.contains(r#"[ -f "$HOME/.bashrc" ] && . "$HOME/.bashrc""#)
        );
        assert!(contents.contains(r#"export PS1="(envx:${ENVX_SHORT}) $PS1""#));
    }

    #[test]
    fn zsh_setup_redirects_zdotdir_and_preserves_original() {
        let setup =
            setup_shell_prompt("/bin/zsh", Vec::new(), PROJECT_ID).unwrap();

        assert_eq!(setup.env.get("ENVX_SHORT"), Some(&"12345678".to_owned()));
        assert!(setup.env.contains_key("ENVX_ORIG_ZDOTDIR"));
        let zdotdir = setup.env.get("ZDOTDIR").unwrap();
        let zshrc = PathBuf::from(zdotdir).join(".zshrc");
        let contents = std::fs::read_to_string(zshrc).unwrap();
        assert!(contents.contains(r#"${ENVX_ORIG_ZDOTDIR:-$HOME}/.zshrc"#));
        assert!(contents.contains(r#"PROMPT="(envx:${ENVX_SHORT}) $PROMPT""#));
    }

    #[test]
    fn fish_setup_wraps_fish_prompt() {
        let setup = setup_shell_prompt("/usr/bin/fish", Vec::new(), PROJECT_ID)
            .unwrap();

        assert_eq!(setup.env.get("ENVX_SHORT"), Some(&"12345678".to_owned()));
        assert_eq!(setup.args[0], "--init-command");
        assert!(setup.args[1]
            .contains("functions -c fish_prompt _envx_old_fish_prompt"));
        assert!(setup.args[1].contains(r#"printf "(envx:%s) " $ENVX_SHORT"#));
    }
}
