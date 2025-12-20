use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::Hash;
use std::io;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use serde::{de::DeserializeOwned, Serialize};

use crate::state::{ExitMode, Tag};

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
            is_ephemeral: false,
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
}
