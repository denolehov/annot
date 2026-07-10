//! Shared test fixtures: hermetic git helpers for fixture-repo tests.
//! Registered in lib.rs under `#[cfg(test)]` — never compiled into the app.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Hermetic git: ignores system/global config, pins identity and autocrlf.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", dir.join("no-such-gitconfig"))
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t.io",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.autocrlf=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Writes `bytes` as a blob into the repo's object store, returns its oid.
pub fn hash_object(dir: &Path, bytes: &[u8]) -> String {
    let mut child = Command::new("git")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", dir.join("no-such-gitconfig"))
        .args(["hash-object", "-w", "--stdin"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run git");
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "git hash-object failed");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}
