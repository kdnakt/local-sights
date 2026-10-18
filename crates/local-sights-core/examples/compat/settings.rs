//! The run settings and the checks of the export folder and the period
//! (261017-insights-query U4:BR1.2-BR1.4, entities.md RunSettings).
//!
//! `insights-export.sh` writes the conditions it ran with to
//! `settings.json` in the export folder, and `compat_check` takes them from
//! there only: it has no arguments for the connection, the log group or the
//! period, so the two cannot be given different conditions (BR1.4, review
//! R-05). The export holds log contents, so its folder must lie outside the
//! repository (BR1.2), and the period must have ended at least 30 minutes
//! before the run, so that late logs do not make the two see different
//! events (BR1.3).

use std::fmt;
use std::path::{Component, Path};

use serde::Deserialize;

/// The period must have ended this many seconds before the run (BR1.3).
pub const MIN_END_AGE_SECONDS: i64 = 30 * 60;

/// The conditions of one comparison (entities.md RunSettings), as
/// `settings.json` holds them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSettings {
    /// The profile; `None` (or blank) for the SDK's default settings.
    #[serde(default)]
    pub profile: Option<String>,
    /// The region code.
    pub region: String,
    /// The log group.
    pub log_group: String,
    /// Start of the period, epoch seconds, given to `start-query` as it is.
    pub start_seconds: i64,
    /// End of the period, epoch seconds, given to `start-query` as it is.
    pub end_seconds: i64,
    /// When the script ran, epoch seconds; every `<id>.json` of that run
    /// carries the same value (code review R-02). `None` when missing: then
    /// no export is taken for this run.
    #[serde(default)]
    pub exported_at_seconds: Option<i64>,
}

/// Why the settings or the export folder cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    /// `settings.json` is not the expected JSON; the parser's reason.
    Unreadable(String),
    /// The region is not a region code such as `us-east-1`.
    BadRegion,
    /// The log group is blank.
    EmptyLogGroup,
    /// The start is negative or not before the end.
    RangeOrder,
    /// BR1.3: the end is later than `latest` (now minus 30 minutes).
    EndTooRecent {
        /// The end, epoch seconds.
        end: i64,
        /// The latest end allowed, epoch seconds.
        latest: i64,
    },
    /// BR1.2: the export folder is inside the repository.
    InsideRepository,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::Unreadable(reason) => {
                write!(f, "settings.json cannot be read: {reason}")
            }
            SettingsError::BadRegion => {
                write!(
                    f,
                    "settings.json: region must be a region code such as us-east-1"
                )
            }
            SettingsError::EmptyLogGroup => write!(f, "settings.json: logGroup is empty"),
            SettingsError::RangeOrder => write!(
                f,
                "settings.json: startSeconds must be 0 or more and before endSeconds"
            ),
            SettingsError::EndTooRecent { end, latest } => write!(
                f,
                "the period must end at least 30 minutes ago (end {end} is after {latest}); \
                 export an earlier period"
            ),
            SettingsError::InsideRepository => write!(
                f,
                "the export folder is inside the repository; use a folder outside it \
                 (the export holds log contents)"
            ),
        }
    }
}

/// Reads and checks `settings.json` (BR1.4).
pub fn parse_settings(json: &str) -> Result<RunSettings, SettingsError> {
    let mut settings: RunSettings =
        serde_json::from_str(json).map_err(|error| SettingsError::Unreadable(error.to_string()))?;
    let region = settings.region.trim();
    let looks_like_a_region = !region.is_empty()
        && region
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !looks_like_a_region {
        return Err(SettingsError::BadRegion);
    }
    settings.region = region.to_string();
    if settings.log_group.trim().is_empty() {
        return Err(SettingsError::EmptyLogGroup);
    }
    if settings.start_seconds < 0 || settings.start_seconds >= settings.end_seconds {
        return Err(SettingsError::RangeOrder);
    }
    // A blank profile is the SDK's default settings, as in fetch_check.
    settings.profile = settings
        .profile
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    Ok(settings)
}

/// BR1.3: the end of the period is at least 30 minutes before `now_seconds`.
pub fn check_end_age(end_seconds: i64, now_seconds: i64) -> Result<(), SettingsError> {
    let latest = now_seconds.saturating_sub(MIN_END_AGE_SECONDS);
    if end_seconds > latest {
        return Err(SettingsError::EndTooRecent {
            end: end_seconds,
            latest,
        });
    }
    Ok(())
}

/// BR1.2, review R-05: whether `folder` is `root` or lies inside it. Both
/// are taken as given (the caller resolves symbolic links first); with
/// `ignore_case` the names are compared without regard to case, as the
/// file system of Windows (and, by default, of macOS) does.
pub fn is_inside(folder: &Path, root: &Path, ignore_case: bool) -> bool {
    let folder = parts(folder, ignore_case);
    let root = parts(root, ignore_case);
    folder.len() >= root.len() && folder[..root.len()] == root[..]
}

/// BR1.2: an error when `folder` is `root` or inside it.
pub fn check_outside(folder: &Path, root: &Path, ignore_case: bool) -> Result<(), SettingsError> {
    if is_inside(folder, root, ignore_case) {
        return Err(SettingsError::InsideRepository);
    }
    Ok(())
}

/// The names of `path`'s parts that count for the comparison.
fn parts(path: &Path, ignore_case: bool) -> Vec<String> {
    path.components()
        .filter(|component| !matches!(component, Component::CurDir))
        .map(|component| {
            let name = component.as_os_str().to_string_lossy();
            if ignore_case {
                name.to_lowercase()
            } else {
                name.into_owned()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const FULL: &str = r#"{"profile":"dev","region":"us-east-1","logGroup":"CloudTrail",
        "startSeconds":1767435900,"endSeconds":1767439500,"exportedAtSeconds":1767450000}"#;

    #[test]
    fn the_settings_are_read_and_the_profile_may_be_left_out() {
        let settings = parse_settings(FULL).unwrap();
        assert_eq!(
            settings,
            RunSettings {
                profile: Some("dev".to_string()),
                region: "us-east-1".to_string(),
                log_group: "CloudTrail".to_string(),
                start_seconds: 1_767_435_900,
                end_seconds: 1_767_439_500,
                exported_at_seconds: Some(1_767_450_000),
            }
        );
        let without = parse_settings(
            r#"{"profile":null,"region":"ap-northeast-1","logGroup":"g","startSeconds":0,"endSeconds":1}"#,
        )
        .unwrap();
        assert_eq!(without.profile, None);
        // Code review R-02: without the stamp no export matches it.
        assert_eq!(without.exported_at_seconds, None);
    }

    #[test]
    fn a_missing_or_wrong_item_of_the_settings_is_an_error() {
        let missing = parse_settings(r#"{"region":"us-east-1","startSeconds":1,"endSeconds":2}"#);
        assert!(
            matches!(&missing, Err(SettingsError::Unreadable(reason)) if reason.contains("logGroup")),
            "{missing:?}"
        );
        let base = |region: &str, group: &str, start: i64, end: i64| {
            format!(
                r#"{{"region":"{region}","logGroup":"{group}","startSeconds":{start},"endSeconds":{end}}}"#
            )
        };
        assert_eq!(
            parse_settings(&base("Tokyo 1", "g", 1, 2)),
            Err(SettingsError::BadRegion)
        );
        assert_eq!(
            parse_settings(&base("us-east-1", "  ", 1, 2)),
            Err(SettingsError::EmptyLogGroup)
        );
        assert_eq!(
            parse_settings(&base("us-east-1", "g", 2, 2)),
            Err(SettingsError::RangeOrder)
        );
        assert_eq!(
            parse_settings(&base("us-east-1", "g", -1, 2)),
            Err(SettingsError::RangeOrder)
        );
        assert!(matches!(
            parse_settings("not json"),
            Err(SettingsError::Unreadable(_))
        ));
    }

    #[test]
    fn an_end_less_than_30_minutes_ago_is_an_error() {
        let now = 1_767_450_000;
        assert_eq!(check_end_age(now - 1800, now), Ok(()));
        assert_eq!(check_end_age(now - 7200, now), Ok(()));
        assert_eq!(
            check_end_age(now - 1799, now),
            Err(SettingsError::EndTooRecent {
                end: now - 1799,
                latest: now - 1800
            })
        );
        assert!(check_end_age(now + 60, now).is_err());
    }

    #[test]
    fn a_folder_inside_the_repository_is_an_error_and_one_outside_is_not() {
        let root = PathBuf::from("/home/dev/local-sights");
        for inside in [
            "/home/dev/local-sights",
            "/home/dev/local-sights/insights-export",
            "/home/dev/local-sights/./target/x",
        ] {
            assert_eq!(
                check_outside(Path::new(inside), &root, false),
                Err(SettingsError::InsideRepository),
                "{inside}"
            );
        }
        for outside in [
            "/home/dev/insights-export",
            "/home/dev/local-sights-export",
            "/tmp/x",
        ] {
            assert_eq!(
                check_outside(Path::new(outside), &root, false),
                Ok(()),
                "{outside}"
            );
        }
    }

    #[test]
    fn a_folder_differing_only_in_case_is_inside_when_case_is_ignored() {
        let root = PathBuf::from("/Users/Dev/Local-Sights");
        let folder = Path::new("/users/dev/local-sights/export");
        assert!(is_inside(folder, &root, true));
        assert!(!is_inside(folder, &root, false));
        assert!(!is_inside(Path::new("/users/dev/export"), &root, true));
    }
}
