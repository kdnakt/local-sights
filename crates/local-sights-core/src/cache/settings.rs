//! The settings file of the disk cache (U6:BR1.1, BR1.2, BR3.6).
//!
//! The file holds exactly one setting, whether the cache is enabled, and the
//! format version: `{"formatVersion":1,"cacheEnabled":true}`. No profile,
//! region, log group or credential is ever written (BR1.1, project.md
//! Forbidden). It is written like a cache file: to a temporary file in the
//! same folder, then renamed (BR3.4), owner-only (BR3.6).

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{CacheError, ensure_private_dir, write_atomically};

/// Version of the settings file format.
pub const SETTINGS_FORMAT_VERSION: u32 = 1;

/// The whole content of the settings file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsFile {
    format_version: u32,
    cache_enabled: bool,
}

/// What reading the settings file found (BR1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsLoad {
    /// There is no settings file (first launch).
    Missing,
    /// The file exists but cannot be read.
    Unreadable,
    /// The file cannot be parsed, or has an unknown format version.
    Invalid,
    /// The saved setting.
    Loaded {
        /// Whether the cache is enabled.
        cache_enabled: bool,
    },
}

impl SettingsLoad {
    /// Whether the cache starts enabled: only when the file says so; every
    /// other case starts disabled (FR7.1, BR1.2).
    pub fn cache_enabled(self) -> bool {
        matches!(
            self,
            SettingsLoad::Loaded {
                cache_enabled: true
            }
        )
    }
}

/// Reads the settings file at `path` (BR1.1, BR1.2). A file that cannot be
/// read or parsed only leaves a diagnostic on standard error; it is not
/// rewritten until the next save.
pub fn load_settings(path: &Path) -> SettingsLoad {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return SettingsLoad::Missing,
        Err(error) => {
            eprintln!(
                "local-sights: settings: the settings file could not be read ({}); the cache stays disabled",
                error.kind()
            );
            return SettingsLoad::Unreadable;
        }
    };
    match serde_json::from_str::<SettingsFile>(&text) {
        Ok(file) if file.format_version == SETTINGS_FORMAT_VERSION => SettingsLoad::Loaded {
            cache_enabled: file.cache_enabled,
        },
        _ => {
            eprintln!(
                "local-sights: settings: the settings file could not be understood; the cache stays disabled"
            );
            SettingsLoad::Invalid
        }
    }
}

/// Writes `cache_enabled` to the settings file at `path` (BR1.1): the
/// folder is created owner-only or tightened (BR3.6), the content goes to a
/// temporary file renamed over the old one (BR3.4).
///
/// # Errors
/// [`CacheError`] when the folder cannot be prepared or the file cannot be
/// written; the previous file is then unchanged.
pub fn save_settings(path: &Path, cache_enabled: bool) -> Result<(), CacheError> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str())) else {
        return Err(CacheError::NotADirectory);
    };
    ensure_private_dir(dir)?;
    let content = SettingsFile {
        format_version: SETTINGS_FORMAT_VERSION,
        cache_enabled,
    };
    let temp_name = format!("{name}.tmp-{:016x}", fastrand::u64(..));
    write_atomically(dir, name, &temp_name, |out| {
        serde_json::to_writer(&mut *out, &content).map_err(io::Error::from)?;
        out.write_all(b"\n")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_path(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("config").join("settings.json")
    }

    #[test]
    fn no_settings_file_means_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let load = load_settings(&settings_path(&dir));
        assert_eq!(load, SettingsLoad::Missing);
        assert!(!load.cache_enabled());
    }

    #[test]
    fn a_saved_setting_is_read_back_and_holds_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir);
        save_settings(&path, true).unwrap();
        assert_eq!(
            load_settings(&path),
            SettingsLoad::Loaded {
                cache_enabled: true
            }
        );
        assert!(load_settings(&path).cache_enabled());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\"formatVersion\":1,\"cacheEnabled\":true}\n"
        );
        save_settings(&path, false).unwrap();
        assert!(!load_settings(&path).cache_enabled());
        let names: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(names.len(), 1, "no temporary file is left");
    }

    #[cfg(unix)]
    #[test]
    fn the_settings_file_and_folder_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir);
        let folder = path.parent().unwrap().to_path_buf();
        fs::create_dir_all(&folder).unwrap();
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
        save_settings(&path, true).unwrap();
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), super::super::PRIVATE_FILE_MODE);
        assert_eq!(mode(&folder), super::super::PRIVATE_DIR_MODE);
    }

    #[test]
    fn a_broken_or_unknown_settings_file_means_disabled_and_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for text in [
            "not json",
            "{\"formatVersion\":2,\"cacheEnabled\":true}",
            "{\"formatVersion\":1}",
            "{\"formatVersion\":1,\"cacheEnabled\":\"yes\"}",
        ] {
            fs::write(&path, text).unwrap();
            assert_eq!(load_settings(&path), SettingsLoad::Invalid, "{text}");
            assert!(!load_settings(&path).cache_enabled());
            assert_eq!(fs::read_to_string(&path).unwrap(), text, "not rewritten");
        }
        // A folder where the file should be cannot be read.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(load_settings(&path), SettingsLoad::Unreadable);
    }

    #[test]
    fn a_settings_file_that_cannot_be_written_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("config");
        fs::write(&folder, b"a regular file").unwrap();
        assert_eq!(
            save_settings(&folder.join("settings.json"), true),
            Err(CacheError::NotADirectory)
        );
        assert_eq!(fs::read(&folder).unwrap(), b"a regular file");
    }
}
