//! Best-effort browser launcher. If exec fails we just print the URL and let
//! the user paste it — never fatal.

use std::process::Command;

pub fn open_url(url: &str) {
    if let Err(e) = spawn(url) {
        eprintln!("annot: couldn't auto-launch browser ({e}); open this URL manually: {url}");
    }
}

#[cfg(target_os = "linux")]
fn spawn(url: &str) -> std::io::Result<()> {
    let cmd = if is_wsl() { "wslview" } else { "xdg-open" };
    Command::new(cmd).arg(url).spawn().map(|_| ())
}

#[cfg(target_os = "macos")]
fn spawn(url: &str) -> std::io::Result<()> {
    Command::new("open").arg(url).spawn().map(|_| ())
}

#[cfg(target_os = "windows")]
fn spawn(url: &str) -> std::io::Result<()> {
    Command::new("cmd").args(["/c", "start", "", url]).spawn().map(|_| ())
}

#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|s| s.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}
