//! Release channel identity.
//!
//! annot ships two channels: `stable` (tagged releases) and `preview` (builds
//! that track `main`). The channel is baked in at compile time from the
//! `ANNOT_CHANNEL` and `ANNOT_BUILD_SHA` environment variables, which CI sets
//! for preview builds. A normal local or release build leaves them unset and
//! is therefore `stable` — behaviour is byte-identical to a build that never
//! had this module.

use std::sync::OnceLock;

use serde::Serialize;

/// Release channel this binary was built for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// Tagged release build. The default for any build without CI overrides.
    #[default]
    Stable,
    /// Build tracking `main`; carries the short commit SHA it was built from.
    Preview(String),
}

/// Raw channel name baked in at compile time. `"stable"` unless CI overrides it.
const CHANNEL: &str = match option_env!("ANNOT_CHANNEL") {
    Some(c) => c,
    None => "stable",
};

/// Short commit SHA this binary was built from. `"dev"` for local builds.
const BUILD_SHA: &str = match option_env!("ANNOT_BUILD_SHA") {
    Some(s) => s,
    None => "dev",
};

/// The channel this binary was built for. Resolved once from compile-time env.
pub fn current() -> &'static Channel {
    static CURRENT: OnceLock<Channel> = OnceLock::new();
    CURRENT.get_or_init(|| match CHANNEL {
        "preview" => Channel::Preview(BUILD_SHA.to_string()),
        _ => Channel::Stable,
    })
}

impl Channel {
    /// Whether this is a non-stable (preview) build.
    pub fn is_preview(&self) -> bool {
        matches!(self, Channel::Preview(_))
    }

    /// Human-facing application name. Preview builds carry a visible suffix so
    /// stable and preview windows are distinguishable at a glance.
    pub fn display_name(&self) -> &'static str {
        match self {
            Channel::Stable => "annot",
            Channel::Preview(_) => "annot (preview)",
        }
    }

    /// Config subdirectory name under the OS config dir. Preview gets its own
    /// directory, isolated from stable's tags, exit modes, and bookmarks.
    pub fn config_subdir(&self) -> &'static str {
        match self {
            Channel::Stable => "annot",
            Channel::Preview(_) => "annot-preview",
        }
    }

    /// Build marker for non-stable builds, e.g. `"preview · g1a2b3c"`.
    /// `None` for stable, so stable output stays unchanged.
    pub fn marker(&self) -> Option<String> {
        match self {
            Channel::Stable => None,
            Channel::Preview(sha) => Some(format!("preview · {sha}")),
        }
    }
}

/// Full version string: the crate version, plus a channel marker for
/// non-stable builds. Used by both `annot version` and clap's `--version`
/// (which requires a `&'static str`), so the result is computed once and cached.
pub fn version_string() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE
        .get_or_init(|| match current().marker() {
            Some(marker) => format!("{} ({})", env!("CARGO_PKG_VERSION"), marker),
            None => env!("CARGO_PKG_VERSION").to_string(),
        })
        .as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_is_the_default() {
        // The test binary compiles without ANNOT_CHANNEL set, so it is stable.
        assert_eq!(*current(), Channel::Stable);
        assert!(!current().is_preview());
        assert_eq!(current().display_name(), "annot");
        assert_eq!(current().config_subdir(), "annot");
        assert_eq!(current().marker(), None);
        assert_eq!(version_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn channels_never_share_a_config_dir() {
        // The isolation invariant, independent of which channel compiled this.
        assert_ne!(
            Channel::Stable.config_subdir(),
            Channel::Preview(String::new()).config_subdir()
        );
    }

    #[test]
    fn preview_carries_sha_in_marker_and_name() {
        let ch = Channel::Preview("g1a2b3c".to_string());
        assert!(ch.is_preview());
        assert_eq!(ch.marker(), Some("preview · g1a2b3c".to_string()));
        assert_eq!(ch.display_name(), "annot (preview)");
    }
}
