//! The settings file of the app (U6:BR1.1, BR1.2, BR3.6; 261022-small-features
//! U1:BR1.4, BR1.5).
//!
//! The file holds the format version and two settings: whether the disk
//! cache is enabled, and the language preference of the screen (`system`,
//! `ja` or `en`): `{"formatVersion":1,"cacheEnabled":true,"language":"ja"}`.
//! No profile, region, log group or credential is ever written (BR1.1,
//! project.md Forbidden). It is written like a cache file: to a temporary
//! file in the same folder, then renamed (BR3.4), owner-only (BR3.6).
//!
//! The `language` field was added by 261022-small-features U1 without a new
//! format version (U1:BR1.5): a file written before it, which has no
//! `language`, reads as `system`, and a file written now is still read by
//! the earlier version, which ignores the field. An unknown spelling also
//! reads as `system` and never touches the cache setting (U1:BR1.4).

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{CacheError, ensure_private_dir, write_atomically};

/// Version of the settings file format.
pub const SETTINGS_FORMAT_VERSION: u32 = 1;

/// The user's language preference (261022-small-features U1, ADR-005): how
/// the screen decides its language, not the language itself. Spelled
/// `system`, `ja` and `en` in the settings file and in the session view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguagePreference {
    /// Follow the OS language (the default, and what an unknown value means).
    #[default]
    System,
    /// Japanese, whatever the OS language.
    Ja,
    /// English, whatever the OS language.
    En,
}

impl LanguagePreference {
    /// The spelling in the settings file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Ja => "ja",
            Self::En => "en",
        }
    }

    /// The preference spelled `text`, exactly (`system`, `ja`, `en`); `None`
    /// for any other spelling.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "system" => Some(Self::System),
            "ja" => Some(Self::Ja),
            "en" => Some(Self::En),
            _ => None,
        }
    }
}

/// The whole content of the settings file. `language` is read as any JSON
/// value so that an unknown spelling or type leaves the rest of the file
/// readable (U1:BR1.4); it is always written (U1:BR1.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsFile {
    format_version: u32,
    cache_enabled: bool,
    #[serde(default)]
    language: Option<serde_json::Value>,
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
    /// The saved settings.
    Loaded {
        /// Whether the cache is enabled.
        cache_enabled: bool,
        /// The language preference; `System` when the file has none or an
        /// unknown one (U1:BR1.4).
        language: LanguagePreference,
    },
}

impl SettingsLoad {
    /// Whether the cache starts enabled: only when the file says so; every
    /// other case starts disabled (FR7.1, BR1.2).
    pub fn cache_enabled(self) -> bool {
        matches!(
            self,
            SettingsLoad::Loaded {
                cache_enabled: true,
                ..
            }
        )
    }

    /// The language preference to start with: the file's when it has a
    /// known one, else `System` (U1:BR1.4).
    pub fn language(self) -> LanguagePreference {
        match self {
            SettingsLoad::Loaded { language, .. } => language,
            SettingsLoad::Missing | SettingsLoad::Unreadable | SettingsLoad::Invalid => {
                LanguagePreference::System
            }
        }
    }
}

/// The language preference a settings file holds: `System` for a missing
/// field, and for an unknown spelling or type, which is said once on
/// standard error (the value is not a secret, U1:BR1.4).
fn language_of(value: Option<&serde_json::Value>) -> LanguagePreference {
    let Some(value) = value else {
        return LanguagePreference::System;
    };
    match value.as_str().and_then(LanguagePreference::parse) {
        Some(language) => language,
        None => {
            eprintln!(
                "local-sights: settings: the language {value} is not known; the OS language is used"
            );
            LanguagePreference::System
        }
    }
}

/// Reads the settings file at `path` (BR1.1, BR1.2, U1:BR1.4). A file that
/// cannot be read or parsed only leaves a diagnostic on standard error; it is
/// not rewritten until the next save.
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
            language: language_of(file.language.as_ref()),
        },
        _ => {
            eprintln!(
                "local-sights: settings: the settings file could not be understood; the cache stays disabled"
            );
            SettingsLoad::Invalid
        }
    }
}

/// Writes `cache_enabled` and `language` to the settings file at `path`
/// (BR1.1, U1:BR1.5): the folder is created owner-only or tightened (BR3.6),
/// the content goes to a temporary file renamed over the old one (BR3.4).
///
/// # Errors
/// [`CacheError`] when the folder cannot be prepared or the file cannot be
/// written; the previous file is then unchanged.
pub fn save_settings(
    path: &Path,
    cache_enabled: bool,
    language: LanguagePreference,
) -> Result<(), CacheError> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str())) else {
        return Err(CacheError::NotADirectory);
    };
    ensure_private_dir(dir)?;
    let content = SettingsFile {
        format_version: SETTINGS_FORMAT_VERSION,
        cache_enabled,
        language: Some(serde_json::Value::from(language.as_str())),
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
        save_settings(&path, true, LanguagePreference::System).unwrap();
        assert_eq!(
            load_settings(&path),
            SettingsLoad::Loaded {
                cache_enabled: true,
                language: LanguagePreference::System,
            }
        );
        assert!(load_settings(&path).cache_enabled());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":\"system\"}\n"
        );
        save_settings(&path, false, LanguagePreference::System).unwrap();
        assert!(!load_settings(&path).cache_enabled());
        let names: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().collect();
        assert_eq!(names.len(), 1, "no temporary file is left");
    }

    // ---- 261022-small-features U1: the language preference (BR1.4, BR1.5) ----

    #[test]
    fn language_is_read_back_and_defaults_to_system() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir);
        for (language, spelling) in [
            (LanguagePreference::Ja, "ja"),
            (LanguagePreference::En, "en"),
            (LanguagePreference::System, "system"),
        ] {
            save_settings(&path, true, language).unwrap();
            let load = load_settings(&path);
            assert_eq!(
                load,
                SettingsLoad::Loaded {
                    cache_enabled: true,
                    language,
                }
            );
            assert_eq!(load.language(), language);
            assert!(load.cache_enabled());
            // NFR5: exactly the three fields, nothing else.
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                format!(
                    "{{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":\"{spelling}\"}}\n"
                )
            );
        }
        assert_eq!(LanguagePreference::default(), LanguagePreference::System);
    }

    #[test]
    fn an_old_settings_file_without_language_means_system() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for (text, cache_enabled) in [
            ("{\"formatVersion\":1,\"cacheEnabled\":true}", true),
            ("{\"formatVersion\":1,\"cacheEnabled\":false}", false),
        ] {
            fs::write(&path, text).unwrap();
            let load = load_settings(&path);
            assert_eq!(
                load,
                SettingsLoad::Loaded {
                    cache_enabled,
                    language: LanguagePreference::System,
                },
                "{text}"
            );
            assert_eq!(load.cache_enabled(), cache_enabled);
            assert_eq!(fs::read_to_string(&path).unwrap(), text, "not rewritten");
        }
    }

    #[test]
    fn an_unknown_language_means_system_and_keeps_the_cache_setting() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for text in [
            "{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":\"fr\"}",
            "{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":\"\"}",
            "{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":\"JA\"}",
            "{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":1}",
            "{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":null}",
        ] {
            fs::write(&path, text).unwrap();
            assert_eq!(
                load_settings(&path),
                SettingsLoad::Loaded {
                    cache_enabled: true,
                    language: LanguagePreference::System,
                },
                "{text}"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), text, "not rewritten");
        }
        fs::write(
            &path,
            "{\"formatVersion\":1,\"cacheEnabled\":false,\"language\":\"fr\"}",
        )
        .unwrap();
        assert_eq!(
            load_settings(&path),
            SettingsLoad::Loaded {
                cache_enabled: false,
                language: LanguagePreference::System,
            }
        );
    }

    #[test]
    fn a_file_with_unknown_fields_is_still_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = settings_path(&dir);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "{\"formatVersion\":1,\"cacheEnabled\":true,\"language\":\"en\",\"extra\":1}",
        )
        .unwrap();
        assert_eq!(
            load_settings(&path),
            SettingsLoad::Loaded {
                cache_enabled: true,
                language: LanguagePreference::En,
            }
        );
    }

    #[test]
    fn settings_load_language_falls_back_to_system() {
        for load in [
            SettingsLoad::Missing,
            SettingsLoad::Unreadable,
            SettingsLoad::Invalid,
        ] {
            assert_eq!(load.language(), LanguagePreference::System, "{load:?}");
            assert!(!load.cache_enabled(), "{load:?}");
        }
        assert_eq!(
            SettingsLoad::Loaded {
                cache_enabled: false,
                language: LanguagePreference::Ja,
            }
            .language(),
            LanguagePreference::Ja
        );
    }

    #[test]
    fn language_preference_round_trips_its_spelling() {
        for (language, spelling) in [
            (LanguagePreference::System, "system"),
            (LanguagePreference::Ja, "ja"),
            (LanguagePreference::En, "en"),
        ] {
            assert_eq!(language.as_str(), spelling);
            assert_eq!(LanguagePreference::parse(spelling), Some(language));
            assert_eq!(
                serde_json::to_string(&language).unwrap(),
                format!("\"{spelling}\"")
            );
            assert_eq!(
                serde_json::from_str::<LanguagePreference>(&format!("\"{spelling}\"")).unwrap(),
                language
            );
        }
        for other in ["", "fr", "JA", "ja-JP", "System"] {
            assert_eq!(LanguagePreference::parse(other), None, "{other}");
            assert!(serde_json::from_str::<LanguagePreference>(&format!("\"{other}\"")).is_err());
        }
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
        save_settings(&path, true, LanguagePreference::System).unwrap();
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
            save_settings(&folder.join("settings.json"), true, LanguagePreference::Ja),
            Err(CacheError::NotADirectory)
        );
        assert_eq!(fs::read(&folder).unwrap(), b"a regular file");
    }
}
