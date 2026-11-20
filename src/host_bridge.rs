use std::env;
#[cfg(not(target_os = "emscripten"))]
use std::process::Command;

use crate::dex_run::DexHostAction;
use crate::error::{Result, WinedroidError};

#[cfg(not(target_os = "emscripten"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct HostCommand {
    program: String,
    arguments: Vec<String>,
}

pub fn run_host_actions(actions: &[DexHostAction]) -> Result<()> {
    for action in actions {
        run_host_action(action)?;
    }

    Ok(())
}

fn run_host_action(action: &DexHostAction) -> Result<()> {
    match action {
        DexHostAction::OpenBrowser { url } => open_browser_url(url),
    }
}

#[cfg(not(target_os = "emscripten"))]
fn open_browser_url(url: &str) -> Result<()> {
    if env::var_os("WINEDROID_DISABLE_HOST_BROWSER_OPEN").is_some() {
        println!("Host browser open disabled");
        println!("  url: {url}");
        return Ok(());
    }

    let command = host_browser_open_command(url)?;
    let status = Command::new(&command.program)
        .args(&command.arguments)
        .status()
        .map_err(|error| {
            WinedroidError::with_source(
                format!("failed to open browser for URL\nurl: {url}"),
                error,
            )
        })?;

    if !status.success() {
        return Err(WinedroidError::new(format!(
            "browser open command failed\nurl: {url}\nstatus: {status}"
        )));
    }

    Ok(())
}

#[cfg(target_os = "emscripten")]
fn open_browser_url(url: &str) -> Result<()> {
    if env::var_os("WINEDROID_DISABLE_HOST_BROWSER_OPEN").is_some() {
        println!("Host browser open disabled");
        println!("  url: {url}");
        return Ok(());
    }

    extern "C" {
        fn winedroid_web_open_url(data: *const u8, length: usize) -> i32;
    }

    let opened = unsafe { winedroid_web_open_url(url.as_ptr(), url.len()) };
    if opened == 0 {
        return Err(WinedroidError::new(format!(
            "browser blocked opening URL\nurl: {url}"
        )));
    }
    Ok(())
}

#[cfg(not(target_os = "emscripten"))]
fn host_browser_open_command(url: &str) -> Result<HostCommand> {
    if cfg!(target_os = "macos") {
        return Ok(HostCommand {
            program: "open".to_owned(),
            arguments: vec![url.to_owned()],
        });
    }

    if cfg!(target_os = "linux") {
        return Ok(HostCommand {
            program: "xdg-open".to_owned(),
            arguments: vec![url.to_owned()],
        });
    }

    if cfg!(target_os = "windows") {
        return Ok(HostCommand {
            program: "rundll32.exe".to_owned(),
            arguments: vec!["url.dll,FileProtocolHandler".to_owned(), url.to_owned()],
        });
    }

    Err(WinedroidError::new(
        "opening browser URLs is not implemented for this host platform\nnext missing primitive: host browser bridge",
    ))
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests {
    use super::*;

    #[test]
    fn browser_open_command_uses_supported_host_tool() {
        let command =
            host_browser_open_command("https://github.com/Darkempire78/OpenCalc").unwrap();

        if cfg!(target_os = "macos") {
            assert_eq!(command.program, "open");
            assert_eq!(
                command.arguments,
                vec!["https://github.com/Darkempire78/OpenCalc".to_owned()]
            );
            return;
        }

        if cfg!(target_os = "linux") {
            assert_eq!(command.program, "xdg-open");
            assert_eq!(
                command.arguments,
                vec!["https://github.com/Darkempire78/OpenCalc".to_owned()]
            );
            return;
        }

        if cfg!(target_os = "windows") {
            assert_eq!(command.program, "rundll32.exe");
            assert_eq!(
                command.arguments,
                vec![
                    "url.dll,FileProtocolHandler".to_owned(),
                    "https://github.com/Darkempire78/OpenCalc".to_owned()
                ]
            );
        }
    }
}
