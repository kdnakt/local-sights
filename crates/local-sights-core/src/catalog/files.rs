//! Reading the AWS shared config and credentials files (BR1.1-BR1.3).
//!
//! Files are located like the AWS CLI does: `AWS_CONFIG_FILE` and
//! `AWS_SHARED_CREDENTIALS_FILE` when set, otherwise `~/.aws/config` and
//! `~/.aws/credentials`. Each file is read line by line and every line is
//! handed to a [`SectionReader`], which keeps only section names and
//! `region` values; credential values are dropped as soon as they are read
//! (BR1.2). A missing file yields no profiles and no notice; a file that
//! exists but cannot be read yields no profiles and a notice naming only the
//! file kind, never its path or content (BR1.3, R-12).

use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use super::{
    ConfigFileKind, ConnectionCatalog, EnvRegionHints, ProfileSections, SectionReader,
    build_profiles,
};

/// Outcome of reading one shared file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileReadOutcome {
    /// The file does not exist (or no location could be determined).
    Missing,
    /// The file was read.
    Read(ProfileSections),
    /// The file exists but could not be read or decoded.
    Unreadable,
}

/// Location of a shared file: the environment variable when set (a leading
/// `~/` is expanded), otherwise the default under the home directory.
pub fn shared_file_path(kind: ConfigFileKind) -> Option<PathBuf> {
    let (variable, default_name) = match kind {
        ConfigFileKind::Config => ("AWS_CONFIG_FILE", "config"),
        ConfigFileKind::Credentials => ("AWS_SHARED_CREDENTIALS_FILE", "credentials"),
    };
    let home = home::home_dir();
    match std::env::var(variable) {
        Ok(value) if !value.trim().is_empty() => match (value.strip_prefix("~/"), home) {
            (Some(rest), Some(home)) => Some(home.join(rest)),
            _ => Some(PathBuf::from(value)),
        },
        _ => home.map(|home| home.join(".aws").join(default_name)),
    }
}

/// Reads one shared file, keeping only what [`SectionReader`] allows.
pub fn read_shared_file(path: Option<&Path>, kind: ConfigFileKind) -> FileReadOutcome {
    let Some(path) = path else {
        return FileReadOutcome::Missing;
    };
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return FileReadOutcome::Missing,
        Err(_) => return FileReadOutcome::Unreadable,
    };
    let mut reader = SectionReader::new(kind);
    for line in BufReader::new(file).lines() {
        match line {
            Ok(line) => reader.feed_line(&line),
            // Not UTF-8, a directory, an I/O error: the file is unreadable.
            Err(_) => return FileReadOutcome::Unreadable,
        }
    }
    FileReadOutcome::Read(reader.finish())
}

/// Region hints from the process environment (BR1.5).
pub fn env_region_hints() -> EnvRegionHints {
    let read = |name: &str| std::env::var(name).ok();
    EnvRegionHints {
        aws_region: read("AWS_REGION"),
        aws_default_region: read("AWS_DEFAULT_REGION"),
        aws_profile: read("AWS_PROFILE"),
    }
}

/// Builds the catalog from the shared files and the environment. Never
/// fails: unreadable files are reported as notices (BR1.3).
pub fn load_catalog() -> ConnectionCatalog {
    let mut unreadable = Vec::new();
    let mut sections =
        |kind: ConfigFileKind| match read_shared_file(shared_file_path(kind).as_deref(), kind) {
            FileReadOutcome::Read(sections) => sections,
            FileReadOutcome::Missing => ProfileSections::default(),
            FileReadOutcome::Unreadable => {
                unreadable.push(kind);
                ProfileSections::default()
            }
        };
    let config = sections(ConfigFileKind::Config);
    let credentials = sections(ConfigFileKind::Credentials);
    let profiles = build_profiles(&config, &credentials, &env_region_hints());
    ConnectionCatalog::new(profiles, unreadable)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::catalog::ProfileSelector;

    fn write_temp(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        file
    }

    fn dummy_secret() -> String {
        format!("{}{}", "Dummy0Secret0Value0For0Tests", "0123456789AB")
    }

    #[test]
    fn missing_file_or_location_is_silent() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-config");
        assert_eq!(
            read_shared_file(Some(&missing), ConfigFileKind::Config),
            FileReadOutcome::Missing
        );
        assert_eq!(
            read_shared_file(None, ConfigFileKind::Config),
            FileReadOutcome::Missing
        );
    }

    #[test]
    fn a_directory_or_non_utf8_file_is_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            read_shared_file(Some(dir.path()), ConfigFileKind::Credentials),
            FileReadOutcome::Unreadable
        );
        let binary = write_temp(&[b'[', 0xff, 0xfe, b']', b'\n']);
        assert_eq!(
            read_shared_file(Some(binary.path()), ConfigFileKind::Config),
            FileReadOutcome::Unreadable
        );
    }

    #[test]
    fn a_readable_file_yields_names_and_regions_only() {
        let text = format!(
            "[profile dev]\nregion = eu-west-1\naws_secret_access_key = {}\n",
            dummy_secret()
        );
        let file = write_temp(text.as_bytes());
        let FileReadOutcome::Read(sections) =
            read_shared_file(Some(file.path()), ConfigFileKind::Config)
        else {
            panic!("expected the file to be read");
        };
        assert_eq!(sections.names(), ["dev"]);
        assert_eq!(sections.region_of("dev"), Some("eu-west-1"));
        assert!(!format!("{sections:?}").contains(&dummy_secret()));
    }

    /// All environment changes happen inside this single test while holding
    /// ENV_LOCK. Only temporary files are read; nothing contacts AWS.
    #[test]
    fn catalog_is_loaded_from_the_environment_locations_and_reports_unreadable_kinds() {
        let _env = crate::test_support::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let config =
            write_temp(b"[default]\nregion = us-west-2\n[profile prod]\nregion = ap-northeast-1\n");
        let credentials_dir = tempfile::tempdir().unwrap();
        // SAFETY: every test in this crate that reads or writes the process
        // environment holds ENV_LOCK, so these writes cannot race.
        unsafe {
            std::env::set_var("AWS_CONFIG_FILE", config.path());
            std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", credentials_dir.path());
            std::env::set_var("AWS_EC2_METADATA_DISABLED", "true");
            for name in ["AWS_REGION", "AWS_DEFAULT_REGION", "AWS_PROFILE"] {
                std::env::remove_var(name);
            }
        }

        assert_eq!(
            shared_file_path(ConfigFileKind::Config).as_deref(),
            Some(config.path())
        );
        let catalog = load_catalog();
        let selectors: Vec<ProfileSelector> =
            catalog.profiles().iter().map(|p| p.selector()).collect();
        assert_eq!(
            selectors,
            vec![
                ProfileSelector::SdkDefault,
                ProfileSelector::Named("default".to_string()),
                ProfileSelector::Named("prod".to_string()),
            ]
        );
        assert_eq!(
            catalog.profiles()[0].default_region.as_deref(),
            Some("us-west-2")
        );
        assert_eq!(catalog.unreadable_files(), [ConfigFileKind::Credentials]);
        let notice = ConfigFileKind::Credentials.unreadable_message_key();
        assert!(!notice.contains(&credentials_dir.path().display().to_string()));

        // SAFETY: as above.
        unsafe {
            std::env::set_var("AWS_REGION", "eu-north-1");
            std::env::set_var("AWS_CONFIG_FILE", "~/no-such-dir/config");
        }
        assert_eq!(env_region_hints().aws_region.as_deref(), Some("eu-north-1"));
        let expanded = shared_file_path(ConfigFileKind::Config).unwrap();
        assert!(!expanded.starts_with("~"));
        assert!(expanded.ends_with("no-such-dir/config"));
        let catalog = load_catalog();
        assert_eq!(
            catalog.profiles().len(),
            1,
            "missing config: only the SDK default"
        );
        assert_eq!(
            catalog.profiles()[0].default_region.as_deref(),
            Some("eu-north-1")
        );
        // SAFETY: as above.
        unsafe {
            std::env::remove_var("AWS_REGION");
        }
    }
}
