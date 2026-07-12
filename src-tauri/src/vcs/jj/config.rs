//! The user's *real* jj configuration.
//!
//! `StackedConfig::with_defaults()` alone is a trap: it loads only jj-lib's
//! own defaults, so `snapshot.auto-track` is missing, `trunk()` and
//! `immutable_heads()` don't exist, and the user's `[revset-aliases]` are
//! invisible. A revset annot resolves would then mean something different
//! from the same revset typed into `jj`. So we replicate jj-cli's layering:
//!
//! ```text
//! lib defaults → cli defaults (we vendor the ones we need) → user files → repo file
//! ```
//!
//! **Read-only by construction, deliberately.** The obvious move — depend on
//! jj-cli and call `ConfigEnv` — is wrong twice over. It writes: the repo layer
//! goes through `SecureConfig`, which on the legacy-migration path does
//! `fs::remove_file(".jj/repo/config.toml")` and symlinks the new location in
//! its place (`lib/src/secure_config.rs`). Opening a *review* must not delete a
//! file inside `.jj`. And it prints: jj-cli's config functions take a `Ui` and
//! emit warnings, but in MCP mode annot's stdout *is* the JSON-RPC transport, so
//! a stray warning would corrupt the protocol. So annot resolves jj's config
//! indirection by hand, reading only. The snapshot stays the one write annot
//! makes.

use std::path::{Path, PathBuf};

use jj_lib::config::{ConfigLayer, ConfigNamePathBuf, ConfigSource, StackedConfig};
use jj_lib::dsl_util::AliasesMap;
use jj_lib::revset::RevsetAliasesMap;
use jj_lib::settings::UserSettings;

use crate::error::AnnotError;

/// jj's own default config, embedded at compile time — the same way jj-cli
/// embeds it (`include_str!` of its `config/*.toml`). Keeping it as a real TOML
/// file, rather than a string literal in Rust, is what makes it diffable against
/// upstream and pasteable back from it. See the file itself for why it exists
/// and how it's kept honest.
const CLI_DEFAULTS: &str = include_str!("jj-defaults.toml");

fn config_err(e: impl std::fmt::Display) -> AnnotError {
    AnnotError::Diff(format!("failed to load jj config: {e}"))
}

/// jj resolves its config directory with `etcetera`'s *base* strategy, which
/// on macOS is `~/.config` — not `~/Library/Application Support`, which is
/// what `dirs::config_dir()` would hand back. Getting this wrong would mean
/// silently ignoring the config file the user actually edits.
fn user_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        dirs::config_dir()
    }
    #[cfg(not(windows))]
    {
        match std::env::var_os("XDG_CONFIG_HOME") {
            Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
            _ => dirs::home_dir().map(|home| home.join(".config")),
        }
    }
}

/// User config paths, lowest precedence first — mirrors jj-cli's
/// `UnresolvedConfigEnv::resolve_user`. `$JJ_CONFIG` (a PATH-separated list)
/// overrides everything, as in jj.
fn user_config_paths() -> Vec<PathBuf> {
    if let Some(paths) = std::env::var_os("JJ_CONFIG") {
        return std::env::split_paths(&paths)
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
    }
    let mut paths = Vec::new();
    let home_config = dirs::home_dir().map(|home| home.join(".jjconfig.toml"));
    let platform_config = user_config_dir().map(|dir| dir.join("jj").join("config.toml"));
    let platform_dir = user_config_dir().map(|dir| dir.join("jj").join("conf.d"));

    // ~/.jjconfig.toml only counts if it exists (or is the only candidate).
    if let Some(path) = home_config {
        if path.exists() || platform_config.is_none() {
            paths.push(path);
        }
    }
    paths.extend(platform_config);
    paths.extend(platform_dir.filter(|p| p.exists()));
    paths
}

/// The repo-local config layer. Since 0.43 jj keeps it outside the repo:
/// `.jj/repo/config-id` names a directory under the user's config dir. The
/// pre-0.43 in-repo `.jj/repo/config.toml` still wins if it's there (jj
/// migrates it lazily; until it does, it's the live file).
fn repo_config_path(repo_dir: &Path) -> Option<PathBuf> {
    let legacy = repo_dir.join("config.toml");
    if legacy.exists() {
        return Some(legacy);
    }
    let id = std::fs::read_to_string(repo_dir.join("config-id")).ok()?;
    let path = user_config_dir()?
        .join("jj")
        .join("repo")
        .join(id.trim())
        .join("config.toml");
    path.exists().then_some(path)
}

/// Load the settings jj itself would use for a repo whose `.jj/repo` is at
/// `repo_dir`.
pub fn load_settings(repo_dir: &Path) -> Result<UserSettings, AnnotError> {
    let mut config = StackedConfig::with_defaults();
    config.add_layer(
        ConfigLayer::parse(ConfigSource::Default, CLI_DEFAULTS).expect("vendored defaults parse"),
    );

    for path in user_config_paths() {
        let result = if path.is_dir() {
            config.load_dir(ConfigSource::User, &path)
        } else if path.exists() {
            config.load_file(ConfigSource::User, &path)
        } else {
            continue;
        };
        result.map_err(config_err)?;
    }

    if let Some(path) = repo_config_path(repo_dir) {
        config
            .load_file(ConfigSource::Repo, path)
            .map_err(config_err)?;
    }

    UserSettings::from_config(config).map_err(config_err)
}

/// Build the revset alias map from the settings' config layers.
///
/// `UserSettings` never surfaces `[revset-aliases]` on its own — jj-cli walks
/// the layers itself, so we do too. Layer order is precedence order: a user's
/// `'trunk()'` overrides the vendored default because its layer comes later.
/// A malformed alias is skipped, not fatal — same as jj, which warns and
/// carries on.
pub fn revset_aliases(settings: &UserSettings) -> RevsetAliasesMap {
    let table_name = ConfigNamePathBuf::from_iter(["revset-aliases"]);
    let mut aliases: RevsetAliasesMap = AliasesMap::new();
    for layer in settings.config().layers() {
        let Ok(Some(table)) = layer.look_up_table(&table_name) else {
            continue;
        };
        for (decl, item) in table.iter() {
            // An alias is either `'f()' = "expr"` or a table with `definition`.
            let (definition, doc) = match item.as_table_like() {
                Some(t) => (
                    t.get("definition").and_then(|i| i.as_str()),
                    t.get("doc").and_then(|i| i.as_str()).map(str::to_owned),
                ),
                None => (item.as_str(), None),
            };
            if let Some(definition) = definition {
                let _ = aliases.insert(decl, definition, doc);
            }
        }
    }
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Every `[revset-aliases]` entry and `[snapshot]` key jj-cli ships, as jj
    /// itself would load them.
    fn jj_defaults() -> StackedConfig {
        let mut config = StackedConfig::empty();
        config.extend_layers(jj_cli::config::default_config_layers());
        config
    }

    fn aliases_of(config: &StackedConfig) -> BTreeMap<String, String> {
        let table_name = ConfigNamePathBuf::from_iter(["revset-aliases"]);
        let mut out = BTreeMap::new();
        for layer in config.layers() {
            let Ok(Some(table)) = layer.look_up_table(&table_name) else {
                continue;
            };
            for (decl, item) in table.iter() {
                let definition = match item.as_table_like() {
                    Some(t) => t.get("definition").and_then(|i| i.as_str()),
                    None => item.as_str(),
                };
                if let Some(definition) = definition {
                    out.insert(decl.to_string(), definition.trim().to_string());
                }
            }
        }
        out
    }

    /// The tripwire. `CLI_DEFAULTS` is a copy of jj-cli's own default config —
    /// necessary because those defaults live in the cli crate, which annot must
    /// not link (it writes to `.jj` and prints to stdout; see the module docs).
    /// A copy is only safe while it stays a copy, so this compares it against
    /// the real thing and fails on any upstream drift: a `trunk()` that means
    /// something different here than in `jj` would be a silent wrong answer,
    /// not a crash.
    ///
    /// If this fails after a jj bump: paste jj's new definitions into
    /// `CLI_DEFAULTS`. That is the whole maintenance burden, and it is now
    /// impossible to forget.
    #[test]
    fn vendored_defaults_match_jj() {
        let ours = {
            let mut c = StackedConfig::empty();
            c.add_layer(ConfigLayer::parse(ConfigSource::Default, CLI_DEFAULTS).unwrap());
            c
        };
        let theirs = jj_defaults();

        assert_eq!(
            aliases_of(&ours),
            aliases_of(&theirs),
            "vendored [revset-aliases] drifted from jj-cli's defaults"
        );

        // The [revsets] table too — `revsets.log` is what short change-id
        // prefixes disambiguate against, so drift there would silently change
        // which prefixes annot accepts.
        let table = ConfigNamePathBuf::from_iter(["revsets"]);
        let revsets_of = |config: &StackedConfig| -> BTreeMap<String, String> {
            let mut out = BTreeMap::new();
            for layer in config.layers() {
                let Ok(Some(t)) = layer.look_up_table(&table) else {
                    continue;
                };
                for (k, v) in t.iter() {
                    if let Some(v) = v.as_str() {
                        out.insert(k.to_string(), v.to_string());
                    }
                }
            }
            out
        };
        assert_eq!(
            revsets_of(&ours),
            revsets_of(&theirs),
            "vendored [revsets] drifted from jj-cli's defaults"
        );

        for key in ["snapshot.auto-track", "snapshot.max-new-file-size"] {
            assert_eq!(
                ours.get::<String>(ConfigNamePathBuf::from_iter(key.split('.')))
                    .ok(),
                theirs
                    .get::<String>(ConfigNamePathBuf::from_iter(key.split('.')))
                    .ok(),
                "vendored `{key}` drifted from jj-cli's default"
            );
        }
    }

    /// The vendored cli defaults must stay parseable and must define the
    /// aliases every real jj config is written against.
    #[test]
    fn cli_defaults_supply_trunk_and_immutable_heads() {
        let mut config = StackedConfig::with_defaults();
        config.add_layer(ConfigLayer::parse(ConfigSource::Default, CLI_DEFAULTS).unwrap());
        let settings = UserSettings::from_config(config).unwrap();

        assert_eq!(settings.get_string("snapshot.auto-track").unwrap(), "all()");
        let aliases = revset_aliases(&settings);
        assert!(aliases.get_function("trunk", 0).is_some());
        assert!(aliases.get_function("immutable_heads", 0).is_some());
        assert!(aliases.get_function("mutable", 0).is_some());
    }
}
