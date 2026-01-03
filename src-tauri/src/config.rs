use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::Hash;
use std::io;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::state::{Bookmark, ExitMode, Tag, TagUsageStats};

/// Current config version. Bump when making breaking changes.
pub const CONFIG_VERSION: u32 = 1;

/// User's theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// Application configuration stored in config.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Schema version for forward compatibility.
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub obsidian: ObsidianConfig,
}

fn default_version() -> u32 {
    CONFIG_VERSION
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            theme: Theme::default(),
            obsidian: ObsidianConfig::default(),
        }
    }
}

/// Obsidian-related configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObsidianConfig {
    /// List of Obsidian vault names.
    #[serde(default)]
    pub vaults: Vec<String>,
}

/// Trait for types that can be merged during concurrent writes.
pub trait Mergeable: Clone {
    type Id: Eq + Hash + Clone;

    fn id(&self) -> Self::Id;

    /// Merge two items with same ID. `self` is from memory, `other` from disk.
    /// Default: memory wins.
    fn merge_with(&self, _other: &Self) -> Self {
        self.clone()
    }
}

impl Mergeable for Tag {
    type Id = String;
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl Mergeable for ExitMode {
    type Id = String;
    fn id(&self) -> String {
        self.id.clone()
    }
}

impl Mergeable for Bookmark {
    type Id = String;
    fn id(&self) -> String {
        self.id.clone()
    }
}

/// Merge two collections, respecting deletions and preferring memory on conflicts.
fn merge_collections<T: Mergeable>(
    memory: Vec<T>,
    disk: Vec<T>,
    deleted_ids: &HashSet<T::Id>,
) -> Vec<T> {
    let mut result: HashMap<T::Id, T> = HashMap::new();

    // Disk items first (excluding deleted)
    for item in disk {
        if !deleted_ids.contains(&item.id()) {
            result.insert(item.id(), item);
        }
    }

    // Memory overlays disk
    for item in memory {
        let id = item.id();
        if let Some(existing) = result.get(&id) {
            result.insert(id, item.merge_with(existing));
        } else {
            result.insert(id, item);
        }
    }

    result.into_values().collect()
}

/// Write content to a file atomically (write to temp, then rename).
fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, content)?;
    fs::rename(&temp, path)?;
    Ok(())
}

/// Acquire exclusive lock, merge with disk, write atomically.
fn save_merged<T: Mergeable + Serialize + DeserializeOwned>(
    config_filename: &str,
    memory: &[T],
    deleted_ids: &HashSet<T::Id>,
) -> io::Result<()> {
    let dir = ensure_config_dir()?;
    let data_path = dir.join(config_filename);
    let lock_path = dir.join(format!("{}.lock", config_filename));

    // Create lock file and acquire exclusive lock
    let lock_file = File::create(&lock_path)?;
    lock_file.lock_exclusive()?;

    // Read current disk state
    let disk: Vec<T> = if data_path.exists() {
        let content = fs::read_to_string(&data_path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        vec![]
    };

    // Merge and write
    let merged = merge_collections(memory.to_vec(), disk, deleted_ids);
    let content = serde_json::to_string_pretty(&merged)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    atomic_write(&data_path, &content)?;

    FileExt::unlock(&lock_file)?;
    Ok(())
}

/// Returns the config directory path: ~/.config/annot/ on Unix, %APPDATA%\annot\ on Windows.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("annot"))
}

/// Ensures the config directory exists.
fn ensure_config_dir() -> io::Result<PathBuf> {
    let dir = config_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "Could not determine config directory")
    })?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Loads tags from ~/.config/annot/tags.json. Returns empty vec if file doesn't exist.
pub fn load_tags() -> Vec<Tag> {
    let Some(dir) = config_dir() else {
        return vec![];
    };

    let path = dir.join("tags.json");
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| vec![]),
        Err(_) => vec![],
    }
}

/// Saves tags to ~/.config/annot/tags.json with locking and merge.
pub fn save_tags(tags: &[Tag], deleted_ids: &HashSet<String>) -> io::Result<()> {
    save_merged("tags.json", tags, deleted_ids)
}

/// Loads exit modes from ~/.config/annot/exit-modes.json. Returns empty vec if file doesn't exist.
pub fn load_exit_modes() -> Vec<ExitMode> {
    let Some(dir) = config_dir() else {
        return vec![];
    };

    let path = dir.join("exit-modes.json");
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| vec![]),
        Err(_) => vec![],
    }
}

/// Saves exit modes to ~/.config/annot/exit-modes.json with locking and merge.
pub fn save_exit_modes(modes: &[ExitMode], deleted_ids: &HashSet<String>) -> io::Result<()> {
    save_merged("exit-modes.json", modes, deleted_ids)
}

/// Loads bookmarks from ~/.config/annot/bookmarks.json. Returns empty vec if file doesn't exist.
pub fn load_bookmarks() -> Vec<Bookmark> {
    let Some(dir) = config_dir() else {
        return vec![];
    };

    let path = dir.join("bookmarks.json");
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| vec![]),
        Err(_) => vec![],
    }
}

/// Saves bookmarks to ~/.config/annot/bookmarks.json with locking and merge.
pub fn save_bookmarks(bookmarks: &[Bookmark], deleted_ids: &HashSet<String>) -> io::Result<()> {
    save_merged("bookmarks.json", bookmarks, deleted_ids)
}

/// Loads tag usage stats from ~/.config/annot/tag-usage.json. Returns default if file doesn't exist.
pub fn load_tag_usage() -> TagUsageStats {
    let Some(dir) = config_dir() else {
        return TagUsageStats::default();
    };

    let path = dir.join("tag-usage.json");
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => TagUsageStats::default(),
    }
}

/// Saves tag usage stats to ~/.config/annot/tag-usage.json with locking and atomic write.
pub fn save_tag_usage(stats: &TagUsageStats) -> io::Result<()> {
    let dir = ensure_config_dir()?;
    let data_path = dir.join("tag-usage.json");
    let lock_path = dir.join("tag-usage.json.lock");

    // Create lock file and acquire exclusive lock
    let lock_file = File::create(&lock_path)?;
    lock_file.lock_exclusive()?;

    // Read current disk state and merge (additive)
    let disk_stats: TagUsageStats = if data_path.exists() {
        let content = fs::read_to_string(&data_path)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        TagUsageStats::default()
    };

    // Merge: add memory counts to disk counts
    let mut merged = disk_stats;
    for (tag_id, mem_usage) in &stats.tags {
        let entry = merged
            .tags
            .entry(tag_id.clone())
            .or_insert_with(|| crate::state::TagUsage::default());
        entry.count += mem_usage.count;
        // Take the more recent last_used
        if mem_usage.last_used > entry.last_used {
            entry.last_used = mem_usage.last_used;
        }
        // Merge by_language counts
        for (lang, count) in &mem_usage.by_language {
            *entry.by_language.entry(lang.clone()).or_insert(0) += count;
        }
    }

    let content = serde_json::to_string_pretty(&merged)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    atomic_write(&data_path, &content)?;

    FileExt::unlock(&lock_file)?;
    Ok(())
}

/// Loads config from ~/.config/annot/config.json. Returns default if file doesn't exist.
pub fn load_config() -> Config {
    let Some(dir) = config_dir() else {
        return Config::default();
    };

    let path = dir.join("config.json");
    match fs::read_to_string(&path) {
        Ok(content) => {
            let mut config: Config = serde_json::from_str(&content).unwrap_or_default();
            // Ensure version is set (for configs created before versioning)
            if config.version == 0 {
                config.version = CONFIG_VERSION;
            }
            config
        }
        Err(_) => Config::default(),
    }
}

/// Saves config to ~/.config/annot/config.json with locking and atomic write.
pub fn save_config(config: &Config) -> io::Result<()> {
    let dir = ensure_config_dir()?;
    let data_path = dir.join("config.json");
    let lock_path = dir.join("config.json.lock");

    // Create lock file and acquire exclusive lock
    let lock_file = File::create(&lock_path)?;
    lock_file.lock_exclusive()?;

    // Ensure version is current
    let mut config = config.clone();
    config.version = CONFIG_VERSION;

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    atomic_write(&data_path, &content)?;

    FileExt::unlock(&lock_file)?;
    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
// CLAUDE CODE COMMAND DISCOVERY
// ════════════════════════════════════════════════════════════════════════════

/// Default color palette for command exit modes.
const COMMAND_COLORS: [&str; 6] = [
    "#8b5cf6", // violet
    "#06b6d4", // cyan
    "#f97316", // orange
    "#ec4899", // pink
    "#14b8a6", // teal
    "#a855f7", // purple
];

/// Discovers Claude Code slash commands from standard locations.
/// Returns a list of ExitMode entries for each discovered command.
///
/// Searches:
/// - `~/.claude/commands/*.md` (global)
/// - `.claude/commands/*.md` (project, relative to cwd)
pub fn discover_commands() -> Vec<ExitMode> {
    let mut commands = Vec::new();
    let mut search_dirs = Vec::new();

    // Global commands: ~/.claude/commands/
    if let Some(home) = dirs::home_dir() {
        search_dirs.push(home.join(".claude").join("commands"));
    }

    // Project commands: .claude/commands/ (relative to cwd)
    if let Ok(cwd) = std::env::current_dir() {
        search_dirs.push(cwd.join(".claude").join("commands"));
    }

    for dir in search_dirs {
        if let Ok(entries) = discover_commands_in_dir(&dir) {
            commands.extend(entries);
        }
    }

    // Assign order values (after transient and persisted modes)
    for (i, mode) in commands.iter_mut().enumerate() {
        mode.order = 1000 + i as u32; // High order to appear after persisted modes
        // Assign color from palette
        mode.color = COMMAND_COLORS[i % COMMAND_COLORS.len()].to_string();
    }

    commands
}

/// Discover commands in a specific directory (recursively).
fn discover_commands_in_dir(dir: &Path) -> io::Result<Vec<ExitMode>> {
    let mut modes = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Ok(modes);
    }

    // Walk directory recursively
    for entry in walkdir(dir)? {
        if entry.extension().map(|e| e == "md").unwrap_or(false) {
            if let Some(mode) = parse_command_file(&entry) {
                modes.push(mode);
            }
        }
    }

    Ok(modes)
}

/// Recursively walk a directory and collect file paths.
fn walkdir(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            files.extend(walkdir(&path)?);
        } else if path.is_file() {
            files.push(path);
        }
    }

    Ok(files)
}

/// Parse a command .md file and extract exit mode data from frontmatter.
fn parse_command_file(path: &Path) -> Option<ExitMode> {
    use crate::state::ExitModeSource;

    let content = fs::read_to_string(path).ok()?;
    let frontmatter = extract_yaml_frontmatter(&content)?;

    // Get command name from file stem (e.g., "design" from "design.md")
    let name = path.file_stem()?.to_string_lossy().to_string();

    // Description becomes instruction
    let description = frontmatter.get("description").cloned()
        .unwrap_or_else(|| format!("Run /{} command", name));

    Some(ExitMode {
        id: format!("cmd-{}", name),
        name: format!("/{}", name),
        color: String::new(), // Will be assigned later
        instruction: description,
        order: 0, // Will be assigned later
        source: ExitModeSource::Command { path: path.to_path_buf() },
    })
}

/// Extract YAML frontmatter from markdown content.
/// Returns key-value pairs from the frontmatter block (between --- delimiters).
fn extract_yaml_frontmatter(content: &str) -> Option<HashMap<String, String>> {
    let content = content.trim_start();

    // Must start with ---
    if !content.starts_with("---") {
        return None;
    }

    // Find the closing ---
    let rest = &content[3..];
    let end_pos = rest.find("\n---")?;
    let yaml_block = &rest[..end_pos];

    // Parse simple key: value pairs (no nested structures)
    let mut map = HashMap::new();
    for line in yaml_block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim();
            // Remove quotes if present
            let value = value.trim_matches('"').trim_matches('\'').to_string();
            map.insert(key, value);
        }
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

// Internal functions that accept explicit paths, used by tests
#[cfg(test)]
fn load_tags_from(path: &std::path::Path) -> Vec<Tag> {
    match fs::read_to_string(path.join("tags.json")) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| vec![]),
        Err(_) => vec![],
    }
}

#[cfg(test)]
fn save_tags_to(path: &std::path::Path, tags: &[Tag]) -> io::Result<()> {
    fs::create_dir_all(path)?;
    let content = serde_json::to_string_pretty(tags)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path.join("tags.json"), content)
}

#[cfg(test)]
fn load_exit_modes_from(path: &std::path::Path) -> Vec<ExitMode> {
    match fs::read_to_string(path.join("exit-modes.json")) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| vec![]),
        Err(_) => vec![],
    }
}

#[cfg(test)]
fn save_exit_modes_to(path: &std::path::Path, modes: &[ExitMode]) -> io::Result<()> {
    fs::create_dir_all(path)?;
    let content = serde_json::to_string_pretty(modes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path.join("exit-modes.json"), content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ExitModeOrigin;
    use tempfile::TempDir;

    #[test]
    fn load_tags_returns_empty_when_file_missing() {
        let temp = TempDir::new().unwrap();
        let tags = load_tags_from(temp.path());
        assert!(tags.is_empty());
    }

    #[test]
    fn save_and_load_tags_roundtrip() {
        let temp = TempDir::new().unwrap();
        let custom_tags = vec![Tag {
            id: "test12345678".into(),
            name: "CUSTOM".into(),
            instruction: "Custom instruction".into(),
        }];

        save_tags_to(temp.path(), &custom_tags).unwrap();
        let loaded = load_tags_from(temp.path());

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "CUSTOM");
    }

    #[test]
    fn load_exit_modes_returns_empty_when_file_missing() {
        let temp = TempDir::new().unwrap();
        let modes = load_exit_modes_from(temp.path());
        assert!(modes.is_empty());
    }

    #[test]
    fn save_and_load_exit_modes_roundtrip() {
        let temp = TempDir::new().unwrap();
        let custom_modes = vec![ExitMode {
            id: "custom123456".into(),
            name: "Custom".into(),
            color: "#ff0000".into(),
            instruction: "Custom mode".into(),
            order: 0,
            source: ExitModeOrigin::Persisted,
        }];

        save_exit_modes_to(temp.path(), &custom_modes).unwrap();
        let loaded = load_exit_modes_from(temp.path());

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Custom");
    }

    #[test]
    fn config_dir_returns_annot_subdirectory() {
        let dir = config_dir();
        assert!(dir.is_some());
        assert!(dir.unwrap().ends_with("annot"));
    }

    #[test]
    fn config_default_has_empty_vaults() {
        let config = Config::default();
        assert!(config.obsidian.vaults.is_empty());
    }

    #[test]
    fn config_default_has_current_version() {
        let config = Config::default();
        assert_eq!(config.version, CONFIG_VERSION);
    }

    #[test]
    fn config_deserializes_with_missing_fields() {
        // Should handle partial JSON gracefully
        let json = r#"{"obsidian": {}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.obsidian.vaults.is_empty());
        assert_eq!(config.version, CONFIG_VERSION); // default version applied

        // Should handle empty JSON
        let json = "{}";
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.obsidian.vaults.is_empty());
    }

    #[test]
    fn config_deserializes_with_explicit_version() {
        let json = r#"{"version": 1, "obsidian": {"vaults": ["test"]}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.obsidian.vaults.len(), 1);
    }

    #[test]
    fn config_roundtrip() {
        let config = Config {
            version: CONFIG_VERSION,
            theme: Theme::Dark,
            obsidian: ObsidianConfig {
                vaults: vec!["Work Notes".into(), "Personal".into()],
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let loaded: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.version, CONFIG_VERSION);
        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.obsidian.vaults.len(), 2);
        assert_eq!(loaded.obsidian.vaults[0], "Work Notes");
        assert_eq!(loaded.obsidian.vaults[1], "Personal");
    }

    #[test]
    fn config_theme_defaults_to_system() {
        let config = Config::default();
        assert_eq!(config.theme, Theme::System);
    }

    #[test]
    fn config_deserializes_without_theme() {
        // Old configs without theme field should default to System
        let json = r#"{"version": 1, "obsidian": {}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.theme, Theme::System);
    }

    #[test]
    fn config_serializes_with_version() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"version\""));
        assert!(json.contains(&format!("{}", CONFIG_VERSION)));
    }

    // ====== YAML Frontmatter Tests ======

    #[test]
    fn extract_yaml_frontmatter_parses_simple() {
        let content = r#"---
description: "My command"
argument-hint: "arg"
---
# Content here
"#;
        let fm = extract_yaml_frontmatter(content).unwrap();
        assert_eq!(fm.get("description"), Some(&"My command".to_string()));
        assert_eq!(fm.get("argument-hint"), Some(&"arg".to_string()));
    }

    #[test]
    fn extract_yaml_frontmatter_handles_no_quotes() {
        let content = r#"---
description: My command without quotes
---
Content
"#;
        let fm = extract_yaml_frontmatter(content).unwrap();
        assert_eq!(fm.get("description"), Some(&"My command without quotes".to_string()));
    }

    #[test]
    fn extract_yaml_frontmatter_returns_none_without_frontmatter() {
        let content = "# Just a heading\nSome content";
        assert!(extract_yaml_frontmatter(content).is_none());
    }

    #[test]
    fn extract_yaml_frontmatter_returns_none_without_closing() {
        let content = r#"---
description: Test
No closing delimiter
"#;
        assert!(extract_yaml_frontmatter(content).is_none());
    }

    #[test]
    fn extract_yaml_frontmatter_ignores_comments() {
        let content = r#"---
# This is a comment
description: Test
---
Content
"#;
        let fm = extract_yaml_frontmatter(content).unwrap();
        assert_eq!(fm.len(), 1);
        assert_eq!(fm.get("description"), Some(&"Test".to_string()));
    }

    // ====== Command Discovery Tests ======

    #[test]
    fn discover_commands_in_empty_dir_returns_empty() {
        let temp = TempDir::new().unwrap();
        let modes = discover_commands_in_dir(temp.path()).unwrap();
        assert!(modes.is_empty());
    }

    #[test]
    fn discover_commands_parses_command_file() {
        let temp = TempDir::new().unwrap();
        let cmd_file = temp.path().join("test-cmd.md");
        fs::write(&cmd_file, r#"---
description: "Test command description"
---
# Test Command
Instructions here
"#).unwrap();

        let modes = discover_commands_in_dir(temp.path()).unwrap();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].name, "/test-cmd");
        assert_eq!(modes[0].instruction, "Test command description");
        assert!(modes[0].is_command());
        assert_eq!(modes[0].command_path(), Some(cmd_file.as_path()));
    }

    #[test]
    fn discover_commands_skips_files_without_frontmatter() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("no-fm.md"), "# Just a file\nNo frontmatter").unwrap();

        let modes = discover_commands_in_dir(temp.path()).unwrap();
        assert!(modes.is_empty());
    }

    #[test]
    fn discover_commands_assigns_colors() {
        let temp = TempDir::new().unwrap();
        for i in 0..3 {
            fs::write(
                temp.path().join(format!("cmd{}.md", i)),
                format!("---\ndescription: Cmd {}\n---\nContent", i)
            ).unwrap();
        }

        // Use discover_commands with explicit dir
        let mut modes = discover_commands_in_dir(temp.path()).unwrap();
        // Manually assign colors like discover_commands does
        for (i, mode) in modes.iter_mut().enumerate() {
            mode.color = COMMAND_COLORS[i % COMMAND_COLORS.len()].to_string();
        }

        assert_eq!(modes.len(), 3);
        // Each should have a color from the palette
        for mode in &modes {
            assert!(!mode.color.is_empty());
            assert!(mode.color.starts_with('#'));
        }
    }
}
