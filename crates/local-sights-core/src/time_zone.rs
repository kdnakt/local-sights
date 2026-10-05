//! Time zone choice and the local time zone (U4, TimeRangeModel).
//!
//! The screen offers two time zones, Local and UTC, and starts with Local
//! on every launch; the choice is never saved (U4:BR1.1). Which IANA zone
//! "Local" means is held by a [`TimeZoneContext`], the boundary that tests
//! replace with a fixed zone so that they never depend on the OS setting
//! (U4:BR3.4).

use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// The time zone of the inputs and of the log list (U4:BR1.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeZoneChoice {
    /// The OS time zone, with its daylight saving rules (the default).
    #[default]
    Local,
    /// Coordinated Universal Time.
    Utc,
}

/// Which zone "Local" stands for in this run of the app (U4:BR3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeZoneContext {
    local: Tz,
}

/// Why the local zone fell back to UTC at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalZoneFallback {
    /// The OS time zone name could not be read (or was blank).
    Unreadable,
    /// The OS gave a name that the bundled IANA data does not know.
    Unknown {
        /// The name the OS gave.
        name: String,
    },
}

/// The local zone chosen at startup, and why it is UTC when it fell back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalZoneResolution {
    /// The context to give AppSession.
    pub context: TimeZoneContext,
    /// Set when the OS zone could not be used and UTC is used instead.
    pub fallback: Option<LocalZoneFallback>,
}

impl LocalZoneFallback {
    /// One line for the diagnostic log (standard error). It contains only
    /// the time zone name, never a credential (project.md Forbidden).
    pub fn diagnostic(&self) -> String {
        match self {
            LocalZoneFallback::Unreadable => {
                "the OS time zone could not be read; Local uses UTC".to_string()
            }
            LocalZoneFallback::Unknown { name } => {
                format!("the OS time zone {name:?} is unknown; Local uses UTC")
            }
        }
    }
}

impl TimeZoneContext {
    /// A context whose local zone is `local`; tests use fixed zones.
    pub fn new(local: Tz) -> Self {
        Self { local }
    }

    /// A context whose local zone is UTC.
    pub fn utc() -> Self {
        Self::new(Tz::UTC)
    }

    /// A context whose local zone is the IANA zone `name` (for example
    /// `America/New_York`), or `None` when the name is unknown.
    pub fn from_name(name: &str) -> Option<Self> {
        name.trim().parse::<Tz>().ok().map(Self::new)
    }

    /// Chooses the local zone from the name the OS gave (U4:BR1.1): a known
    /// name is used; a missing, blank or unknown name falls back to UTC and
    /// says why, so the caller can log it.
    pub fn resolve(os_name: Option<&str>) -> LocalZoneResolution {
        let name = os_name.map(str::trim).filter(|name| !name.is_empty());
        let Some(name) = name else {
            return LocalZoneResolution {
                context: Self::utc(),
                fallback: Some(LocalZoneFallback::Unreadable),
            };
        };
        match Self::from_name(name) {
            Some(context) => LocalZoneResolution {
                context,
                fallback: None,
            },
            None => LocalZoneResolution {
                context: Self::utc(),
                fallback: Some(LocalZoneFallback::Unknown {
                    name: name.to_string(),
                }),
            },
        }
    }

    /// Reads the OS time zone name once (at startup) and resolves it. The
    /// name comes from `iana-time-zone`; the conversion uses the IANA data
    /// bundled with `chrono-tz`, so the screen and the library never
    /// disagree about Local (U4:BR3.4). Not used by the automated tests.
    pub fn detect() -> LocalZoneResolution {
        let name = iana_time_zone::get_timezone().ok();
        Self::resolve(name.as_deref())
    }

    /// The IANA name of the local zone.
    pub fn local_name(&self) -> &'static str {
        self.local.name()
    }

    /// The zone used as "Local".
    pub fn local(&self) -> Tz {
        self.local
    }

    /// The zone that `choice` stands for.
    pub fn zone(&self, choice: TimeZoneChoice) -> Tz {
        match choice {
            TimeZoneChoice::Local => self.local,
            TimeZoneChoice::Utc => Tz::UTC,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choice_starts_as_local_and_serializes_by_name() {
        assert_eq!(TimeZoneChoice::default(), TimeZoneChoice::Local);
        assert_eq!(
            serde_json::to_value(TimeZoneChoice::Utc).unwrap(),
            serde_json::json!("Utc")
        );
        let local: TimeZoneChoice = serde_json::from_str("\"Local\"").unwrap();
        assert_eq!(local, TimeZoneChoice::Local);
        assert!(serde_json::from_str::<TimeZoneChoice>("\"Asia/Tokyo\"").is_err());
    }

    #[test]
    fn a_known_os_name_becomes_the_local_zone() {
        let resolution = TimeZoneContext::resolve(Some("America/New_York"));
        assert_eq!(resolution.fallback, None);
        assert_eq!(resolution.context.local_name(), "America/New_York");
        assert_eq!(
            resolution.context.zone(TimeZoneChoice::Local),
            chrono_tz::America::New_York
        );
        assert_eq!(resolution.context.zone(TimeZoneChoice::Utc), Tz::UTC);
        assert_eq!(
            TimeZoneContext::from_name(" Asia/Tokyo "),
            Some(TimeZoneContext::new(chrono_tz::Asia::Tokyo))
        );
    }

    #[test]
    fn an_unknown_os_name_falls_back_to_utc_and_says_only_the_name() {
        let resolution = TimeZoneContext::resolve(Some("Mars/Olympus_Mons"));
        assert_eq!(resolution.context, TimeZoneContext::utc());
        let fallback = resolution.fallback.unwrap();
        assert_eq!(
            fallback,
            LocalZoneFallback::Unknown {
                name: "Mars/Olympus_Mons".to_string()
            }
        );
        assert_eq!(
            fallback.diagnostic(),
            "the OS time zone \"Mars/Olympus_Mons\" is unknown; Local uses UTC"
        );
        assert_eq!(TimeZoneContext::from_name("Mars/Olympus_Mons"), None);
    }

    #[test]
    fn a_missing_or_blank_os_name_falls_back_to_utc() {
        for name in [None, Some(""), Some("   ")] {
            let resolution = TimeZoneContext::resolve(name);
            assert_eq!(resolution.context.zone(TimeZoneChoice::Local), Tz::UTC);
            assert_eq!(resolution.fallback, Some(LocalZoneFallback::Unreadable));
        }
        assert_eq!(
            LocalZoneFallback::Unreadable.diagnostic(),
            "the OS time zone could not be read; Local uses UTC"
        );
    }
}
