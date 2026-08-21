//! HookStat-owned persisted Human interface preferences.
//!
//! Reads never create state. Explicit saves use a byte snapshot compare and an
//! atomic replacement so a stale TUI draft cannot overwrite another writer.

use crate::tui::localization::InterfaceLanguage;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InterfaceColor {
    #[default]
    Auto,
    Always,
    Never,
}

impl InterfaceColor {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "always" => Some(Self::Always),
            "never" => Some(Self::Never),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum InterfacePreferencesError {
    Io(io::Error),
    Malformed,
    SymbolicLink,
}

impl fmt::Display for InterfacePreferencesError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Io(_) => "HookStat interface preference storage is unavailable",
            Self::Malformed => "HookStat interface preference data is malformed",
            Self::SymbolicLink => "HookStat interface preference file is a symbolic link",
        })
    }
}

impl std::error::Error for InterfacePreferencesError {}

impl From<io::Error> for InterfacePreferencesError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfacePreferenceSnapshot {
    language: InterfaceLanguage,
    color: InterfaceColor,
    bytes: Option<Vec<u8>>,
}

impl InterfacePreferenceSnapshot {
    pub const fn language(&self) -> InterfaceLanguage {
        self.language
    }

    pub const fn color(&self) -> InterfaceColor {
        self.color
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreferenceSaveOutcome {
    Saved,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfacePreferencesStore {
    path: PathBuf,
}

impl InterfacePreferencesStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads without creating the parent directory or preference file.
    pub fn snapshot_read_only(
        &self,
    ) -> Result<InterfacePreferenceSnapshot, InterfacePreferencesError> {
        reject_symbolic_link(&self.path)?;
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let (language, color) = bytes
            .as_deref()
            .map(preferences_from_bytes)
            .transpose()?
            .unwrap_or((InterfaceLanguage::Auto, InterfaceColor::Auto));
        Ok(InterfacePreferenceSnapshot {
            language,
            color,
            bytes,
        })
    }

    /// Saves only if the exact read-only document remains current. Unknown TOML
    /// fields are retained, and only this HookStat-owned file may be written.
    pub fn save_if_unchanged(
        &self,
        expected: &InterfacePreferenceSnapshot,
        language: InterfaceLanguage,
        color: InterfaceColor,
    ) -> Result<PreferenceSaveOutcome, InterfacePreferencesError> {
        let parent = self
            .path
            .parent()
            .ok_or(InterfacePreferencesError::Malformed)?;
        let Some(_lock) = PreferenceLock::acquire(parent)? else {
            return Ok(PreferenceSaveOutcome::Conflict);
        };
        let current = self.snapshot_read_only()?;
        if current.bytes != expected.bytes {
            return Ok(PreferenceSaveOutcome::Conflict);
        }
        let mut document = match current.bytes.as_deref() {
            Some(bytes) => std::str::from_utf8(bytes)
                .map_err(|_| InterfacePreferencesError::Malformed)?
                .parse::<toml::Table>()
                .map_err(|_| InterfacePreferencesError::Malformed)?,
            None => toml::Table::new(),
        };
        let interface = document
            .entry("interface")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        let interface = interface
            .as_table_mut()
            .ok_or(InterfacePreferencesError::Malformed)?;
        interface.insert(
            "language".into(),
            toml::Value::String(language.as_storage().into()),
        );
        interface.insert(
            "color".into(),
            toml::Value::String(color.as_storage().into()),
        );
        let bytes = toml::to_string(&toml::Value::Table(document))
            .map_err(|_| InterfacePreferencesError::Malformed)?
            .into_bytes();
        atomic_write(&self.path, &bytes)?;
        Ok(PreferenceSaveOutcome::Saved)
    }
}

fn preferences_from_bytes(
    bytes: &[u8],
) -> Result<(InterfaceLanguage, InterfaceColor), InterfacePreferencesError> {
    let document = std::str::from_utf8(bytes)
        .map_err(|_| InterfacePreferencesError::Malformed)?
        .parse::<toml::Table>()
        .map_err(|_| InterfacePreferencesError::Malformed)?;
    let Some(interface) = document.get("interface") else {
        return Ok((InterfaceLanguage::Auto, InterfaceColor::Auto));
    };
    let Some(table) = interface.as_table() else {
        return Err(InterfacePreferencesError::Malformed);
    };
    let language = table
        .get("language")
        .map_or(Some(InterfaceLanguage::Auto), |value| {
            value.as_str().and_then(InterfaceLanguage::parse)
        })
        .ok_or(InterfacePreferencesError::Malformed)?;
    let color = table
        .get("color")
        .map_or(Some(InterfaceColor::Auto), |value| {
            value.as_str().and_then(InterfaceColor::parse)
        })
        .ok_or(InterfacePreferencesError::Malformed)?;
    Ok((language, color))
}

struct PreferenceLock {
    path: PathBuf,
}

impl PreferenceLock {
    fn acquire(parent: &Path) -> Result<Option<Self>, InterfacePreferencesError> {
        fs::create_dir_all(parent)?;
        let path = parent.join(".interface.lock");
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => Ok(Some(Self { path })),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for PreferenceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), InterfacePreferencesError> {
    let parent = path.parent().ok_or(InterfacePreferencesError::Malformed)?;
    fs::create_dir_all(parent)?;
    reject_symbolic_link(path)?;
    let sequence = WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".interface-{}-{sequence}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let write_result = (|| -> Result<(), io::Error> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result.map_err(Into::into)
}

fn reject_symbolic_link(path: &Path) -> Result<(), InterfacePreferencesError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(InterfacePreferencesError::SymbolicLink)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn absent_read_does_not_create_interface_state() {
        let temporary = tempdir().unwrap();
        let store = InterfacePreferencesStore::new(temporary.path().join("state/interface.toml"));
        let snapshot = store.snapshot_read_only().unwrap();
        assert_eq!(snapshot.language(), InterfaceLanguage::Auto);
        assert_eq!(snapshot.color(), InterfaceColor::Auto);
        assert!(!store.path().exists());
        assert!(!store.path().parent().unwrap().exists());
    }

    #[test]
    fn save_preserves_unknown_fields_and_rejects_a_stale_snapshot() {
        let temporary = tempdir().unwrap();
        let path = temporary.path().join("interface.toml");
        fs::write(
            &path,
            "future = 1\n[interface]\ncolor = 'auto'\nlanguage = 'en-US'\n",
        )
        .unwrap();
        let store = InterfacePreferencesStore::new(&path);
        let snapshot = store.snapshot_read_only().unwrap();
        assert_eq!(snapshot.language(), InterfaceLanguage::EnUs);
        assert_eq!(snapshot.color(), InterfaceColor::Auto);
        assert_eq!(
            store
                .save_if_unchanged(&snapshot, InterfaceLanguage::ZhCn, InterfaceColor::Never)
                .unwrap(),
            PreferenceSaveOutcome::Saved
        );
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("future = 1"));
        assert!(saved.contains("color = \"never\""));
        fs::write(&path, "[interface]\nlanguage = 'auto'\n").unwrap();
        assert_eq!(
            store
                .save_if_unchanged(&snapshot, InterfaceLanguage::EnUs, InterfaceColor::Always)
                .unwrap(),
            PreferenceSaveOutcome::Conflict
        );
    }

    #[test]
    fn active_preference_lock_refuses_a_concurrent_apply_without_overwrite() {
        let temporary = tempdir().unwrap();
        let path = temporary.path().join("interface.toml");
        let store = InterfacePreferencesStore::new(&path);
        let snapshot = store.snapshot_read_only().unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let lock = path.parent().unwrap().join(".interface.lock");
        fs::write(&lock, "held").unwrap();
        assert_eq!(
            store
                .save_if_unchanged(&snapshot, InterfaceLanguage::ZhCn, InterfaceColor::Never)
                .unwrap(),
            PreferenceSaveOutcome::Conflict
        );
        assert!(!path.exists());
        fs::remove_file(lock).unwrap();
    }
}
