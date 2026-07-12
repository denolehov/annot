//! Shared test fixtures: hermetic git and jj helpers for fixture-repo tests.
//! Registered in lib.rs under `#[cfg(test)]` — never compiled into the app.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

/// Run hermetic git, ignoring system/global config and pinning identity and
/// autocrlf. Unlike [`git`], this returns non-zero exits to the caller.
pub fn git_output(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
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
        .expect("failed to run git")
}

/// Hermetic git: asserts success and returns trimmed stdout.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = git_output(dir, args);
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Point `$JJ_CONFIG` at a fixture config, once per test process.
///
/// Both the jj CLI (used to build fixtures) and annot's own config loader read
/// `$JJ_CONFIG`, so this makes tests hermetic against the developer's real
/// `~/.config/jj/config.toml` — a machine with, say, `snapshot.auto-track =
/// "none()"` would otherwise fail them. `OnceLock` orders the single write
/// before every read: any thread that reads the var has called this first.
fn hermetic_jj_config() -> &'static PathBuf {
    static CONFIG: OnceLock<PathBuf> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let path = std::env::temp_dir().join("annot-test-jjconfig.toml");
        std::fs::write(
            &path,
            "[user]\nname = \"t\"\nemail = \"t@t.io\"\n[ui]\npaginate = \"never\"\n",
        )
        .expect("failed to write jj test config");
        std::env::set_var("JJ_CONFIG", &path);
        path
    })
}

/// Hermetic jj: asserts success and returns trimmed stdout.
pub fn jj(dir: &Path, args: &[&str]) -> String {
    hermetic_jj_config();
    let out = Command::new("jj")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run jj");
    assert!(
        out.status.success(),
        "jj {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// A jj repo with no git worktree at all. Colocation is jj 0.43's default, so
/// this needs `--no-colocate` — and it is the shape annot used to fail on
/// outright: `gix::discover` finds nothing, because there is no `.git`.
pub fn jj_repo() -> tempfile::TempDir {
    hermetic_jj_config();
    let dir = tempfile::tempdir().unwrap();
    jj(dir.path(), &["git", "init", "--no-colocate"]);
    dir
}

/// A jj repo colocated with git — both `.jj` and `.git` are present.
pub fn jj_colocated_repo() -> tempfile::TempDir {
    hermetic_jj_config();
    let dir = tempfile::tempdir().unwrap();
    jj(dir.path(), &["git", "init", "--colocate"]);
    dir
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
