//! ConnectionCatalog: profiles and regions to choose from (U2).
//!
//! The profile list is built from the AWS shared config and credentials
//! files (BR1.1). Only section headers and `region` lines are looked at;
//! every other line, including credential values, is discarded without
//! being kept (BR1.2). Since 261011-sso-login the config file's profiles
//! also tell whether they have SSO settings (`sso_session` or
//! `sso_start_url`, presence only) and the name of their `source_profile`
//! ([`ProfileAuthInfo`], that intent's BR1.1, BR1.2). This module holds the pure logic; reading the files
//! is in [`files`], the static region list in [`regions`].

pub mod files;
pub mod regions;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Identifies a profile choice: the SDK default chain, or a named profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "profileName")]
pub enum ProfileSelector {
    /// "Default settings (left to the SDK)": no profile is specified.
    SdkDefault,
    /// A profile defined in the shared config or credentials file.
    Named(String),
}

/// Kind of a profile row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProfileKind {
    /// The SDK default chain (always first, exactly once).
    SdkDefault,
    /// A named profile.
    Named,
}

/// One row of the profile list. Holds no credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    /// SdkDefault or Named.
    pub kind: ProfileKind,
    /// Profile name, only for Named.
    pub profile_name: Option<String>,
    /// Default region of the profile (BR1.5), if any.
    pub default_region: Option<String>,
}

/// Which shared file a section came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConfigFileKind {
    /// `~/.aws/config` (or `AWS_CONFIG_FILE`).
    Config,
    /// `~/.aws/credentials` (or `AWS_SHARED_CREDENTIALS_FILE`).
    Credentials,
}

/// Profile names, `region` values, whether a profile has SSO settings and
/// the name of its `source_profile`, read from one file. Nothing else
/// (BR1.2 of 261011-sso-login).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileSections {
    names: Vec<String>,
    regions: BTreeMap<String, String>,
    /// Profiles with an `sso_session` or `sso_start_url` key; the value is
    /// never kept (BR1.1).
    sso_profiles: BTreeSet<String>,
    /// `source_profile` of a profile: a profile name only (BR1.2).
    source_profiles: BTreeMap<String, String>,
}

/// What the login decision needs of one profile (entities.md
/// ProfileAuthInfo): whether it has SSO settings and the name of the
/// profile it takes its credentials from. Holds no secret and no SSO value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileAuthInfo {
    /// The profile name.
    pub profile_name: String,
    /// Whether the section has `sso_session` or `sso_start_url` (BR1.1).
    pub has_sso_settings: bool,
    /// The `source_profile` name, if any (BR1.2).
    pub source_profile: Option<String>,
}

/// Reads a shared file line by line, keeping only section names and (for
/// the config file) `region` values, the presence of SSO settings and the
/// `source_profile` name. Each line is inspected and dropped.
#[derive(Debug, Clone)]
pub struct SectionReader {
    file: ConfigFileKind,
    current: Option<String>,
    sections: ProfileSections,
}

/// Region-related environment variables, passed in explicitly so the pure
/// logic never reads the process environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvRegionHints {
    /// `AWS_REGION`.
    pub aws_region: Option<String>,
    /// `AWS_DEFAULT_REGION`.
    pub aws_default_region: Option<String>,
    /// `AWS_PROFILE`.
    pub aws_profile: Option<String>,
}

/// Profiles, region options and unreadable-file notices for the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionCatalog {
    profiles: Vec<ConnectionProfile>,
    regions: Vec<String>,
    unreadable_files: Vec<ConfigFileKind>,
    /// Login-related settings of the config file's profiles, by name.
    auth: BTreeMap<String, ProfileAuthInfo>,
}

impl ProfileSelector {
    /// Profile name to pass to the SDK (`None` for the SDK default).
    pub fn profile_name(&self) -> Option<&str> {
        match self {
            ProfileSelector::SdkDefault => None,
            ProfileSelector::Named(name) => Some(name),
        }
    }

    /// Selector for an optional, untrimmed name: blank means SDK default.
    pub fn from_optional_name(name: Option<&str>) -> Self {
        match name.map(str::trim) {
            Some(name) if !name.is_empty() => ProfileSelector::Named(name.to_string()),
            _ => ProfileSelector::SdkDefault,
        }
    }
}

impl ConnectionProfile {
    /// The selector that identifies this row.
    pub fn selector(&self) -> ProfileSelector {
        match (&self.kind, &self.profile_name) {
            (ProfileKind::Named, Some(name)) => ProfileSelector::Named(name.clone()),
            _ => ProfileSelector::SdkDefault,
        }
    }
}

impl ConfigFileKind {
    /// Message-catalog key of the "could not read" notice (BR1.3, R-12):
    /// it names the file kind only, never a path or content.
    pub fn unreadable_message_key(self) -> &'static str {
        match self {
            ConfigFileKind::Config => "catalog.unreadable.config",
            ConfigFileKind::Credentials => "catalog.unreadable.credentials",
        }
    }
}

impl ProfileSections {
    /// Profile names in file order (may contain duplicates).
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// `region` of a profile, if the file set one.
    pub fn region_of(&self, profile_name: &str) -> Option<&str> {
        self.regions.get(profile_name).map(String::as_str)
    }

    /// Whether the profile has `sso_session` or `sso_start_url` (BR1.1).
    pub fn has_sso_settings(&self, profile_name: &str) -> bool {
        self.sso_profiles.contains(profile_name)
    }

    /// `source_profile` of a profile, if the file set one (BR1.2).
    pub fn source_profile_of(&self, profile_name: &str) -> Option<&str> {
        self.source_profiles.get(profile_name).map(String::as_str)
    }

    /// The login-related settings of every profile of this file, in name
    /// order, each name once.
    pub fn auth_infos(&self) -> Vec<ProfileAuthInfo> {
        let names: BTreeSet<&String> = self.names.iter().collect();
        names
            .into_iter()
            .map(|name| ProfileAuthInfo {
                profile_name: name.clone(),
                has_sso_settings: self.has_sso_settings(name),
                source_profile: self.source_profile_of(name).map(str::to_string),
            })
            .collect()
    }
}

impl SectionReader {
    /// A reader for the given file kind.
    pub fn new(file: ConfigFileKind) -> Self {
        Self {
            file,
            current: None,
            sections: ProfileSections::default(),
        }
    }

    /// Inspects one line and keeps only what is allowed.
    pub fn feed_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            return;
        }
        if line.starts_with('[') {
            self.current = line
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
                .and_then(|header| profile_name_of_header(header, self.file));
            if let Some(name) = &self.current {
                self.sections.names.push(name.clone());
            }
            return;
        }
        // Only the config file's `region`, whether `sso_session` or
        // `sso_start_url` is there (never its value) and the name in
        // `source_profile` are kept; every other line (credentials, other
        // SSO settings, ...) is dropped right here (BR1.2).
        if self.file != ConfigFileKind::Config {
            return;
        }
        let Some((key, value)) = line.split_once('=') else {
            return;
        };
        let Some(profile) = &self.current else {
            return;
        };
        let value = value.trim();
        match key.trim() {
            "sso_session" | "sso_start_url" => {
                self.sections.sso_profiles.insert(profile.clone());
            }
            "region" if !value.is_empty() => {
                self.sections
                    .regions
                    .insert(profile.clone(), value.to_string());
            }
            "source_profile" if !value.is_empty() => {
                self.sections
                    .source_profiles
                    .insert(profile.clone(), value.to_string());
            }
            _ => {}
        }
    }

    /// The collected sections.
    pub fn finish(self) -> ProfileSections {
        self.sections
    }
}

/// Parses a whole file text (used by tests and by callers holding a string).
pub fn parse_sections(text: &str, file: ConfigFileKind) -> ProfileSections {
    let mut reader = SectionReader::new(file);
    for line in text.lines() {
        reader.feed_line(line);
    }
    reader.finish()
}

/// Profile name of a section header, or `None` for non-profile sections
/// (e.g. `sso-session`, `services`) and malformed headers.
fn profile_name_of_header(header: &str, file: ConfigFileKind) -> Option<String> {
    let words: Vec<&str> = header.split_whitespace().collect();
    let name = match (file, words.as_slice()) {
        (ConfigFileKind::Config, ["default"]) => "default",
        (ConfigFileKind::Config, ["profile", name]) => name,
        (ConfigFileKind::Credentials, [name]) => name,
        _ => return None,
    };
    Some(name.to_string())
}

/// A trimmed, non-empty environment value.
fn non_blank(value: Option<&String>) -> Option<&str> {
    value.map(|v| v.trim()).filter(|v| !v.is_empty())
}

/// Default region of the SDK default chain (BR1.5): `AWS_REGION`, then
/// `AWS_DEFAULT_REGION`, then the region of `AWS_PROFILE` when it is set,
/// otherwise the region of `[default]`.
fn sdk_default_region(config: &ProfileSections, env: &EnvRegionHints) -> Option<String> {
    if let Some(region) =
        non_blank(env.aws_region.as_ref()).or_else(|| non_blank(env.aws_default_region.as_ref()))
    {
        return Some(region.to_string());
    }
    let profile = non_blank(env.aws_profile.as_ref()).unwrap_or("default");
    config.region_of(profile).map(str::to_string)
}

/// Builds the profile list: SdkDefault first, then the unique names of both
/// files in ascending order (BR1.1), each with its default region (BR1.5).
pub fn build_profiles(
    config: &ProfileSections,
    credentials: &ProfileSections,
    env: &EnvRegionHints,
) -> Vec<ConnectionProfile> {
    let mut names: Vec<&String> = config.names().iter().chain(credentials.names()).collect();
    names.sort();
    names.dedup();
    let mut profiles = vec![ConnectionProfile {
        kind: ProfileKind::SdkDefault,
        profile_name: None,
        default_region: sdk_default_region(config, env),
    }];
    profiles.extend(names.into_iter().map(|name| ConnectionProfile {
        kind: ProfileKind::Named,
        profile_name: Some(name.clone()),
        default_region: config.region_of(name).map(str::to_string),
    }));
    profiles
}

impl ConnectionCatalog {
    /// A catalog from built profiles and the files that could not be read.
    /// Region options are derived from [`regions::region_options`].
    pub fn new(profiles: Vec<ConnectionProfile>, unreadable_files: Vec<ConfigFileKind>) -> Self {
        let regions = regions::region_options(&profiles);
        Self {
            profiles,
            regions,
            unreadable_files,
            auth: BTreeMap::new(),
        }
    }

    /// The same catalog with the login-related settings of the config
    /// file's profiles (261011-sso-login BR1.1, BR1.2).
    pub fn with_auth_infos(mut self, infos: Vec<ProfileAuthInfo>) -> Self {
        self.auth = infos
            .into_iter()
            .map(|info| (info.profile_name.clone(), info))
            .collect();
        self
    }

    /// The login-related settings of a profile, when the config file has it.
    pub fn auth_info(&self, profile_name: &str) -> Option<&ProfileAuthInfo> {
        self.auth.get(profile_name)
    }

    /// Profile rows, SdkDefault first.
    pub fn profiles(&self) -> &[ConnectionProfile] {
        &self.profiles
    }

    /// Region codes in ascending order.
    pub fn regions(&self) -> &[String] {
        &self.regions
    }

    /// Files that exist but could not be read or parsed (BR1.3).
    pub fn unreadable_files(&self) -> &[ConfigFileKind] {
        &self.unreadable_files
    }

    /// The row for a selector, if it is in the list.
    pub fn find(&self, selector: &ProfileSelector) -> Option<&ConnectionProfile> {
        self.profiles
            .iter()
            .find(|profile| &profile.selector() == selector)
    }

    /// Whether the region code is one of the options.
    pub fn has_region(&self, code: &str) -> bool {
        self.regions.iter().any(|region| region == code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_secret() -> String {
        format!("{}{}", "Dummy0Secret0Value0For0Tests", "0123456789AB")
    }

    fn dummy_key_id() -> String {
        format!("{}{}", "AKIA", "EXAMPLE0DUMMY000")
    }

    fn config_text() -> String {
        format!(
            "# comment\n[default]\nregion = us-west-2\noutput = json\n\n\
             [profile prod]\nregion=ap-northeast-1\nsso_start_url = https://example.invalid/start\n\
             [profile  dev ]\naws_secret_access_key = {}\n\
             [sso-session corp]\nregion = eu-west-1\n",
            dummy_secret()
        )
    }

    fn credentials_text() -> String {
        format!(
            "[dev]\naws_access_key_id = {}\naws_secret_access_key = {}\nregion = sa-east-1\n\
             [ci]\naws_access_key_id = {}\n",
            dummy_key_id(),
            dummy_secret(),
            dummy_key_id()
        )
    }

    fn named(name: &str, region: Option<&str>) -> ConnectionProfile {
        ConnectionProfile {
            kind: ProfileKind::Named,
            profile_name: Some(name.to_string()),
            default_region: region.map(str::to_string),
        }
    }

    fn sdk_default(region: Option<&str>) -> ConnectionProfile {
        ConnectionProfile {
            kind: ProfileKind::SdkDefault,
            profile_name: None,
            default_region: region.map(str::to_string),
        }
    }

    #[test]
    fn config_sections_and_regions_are_collected() {
        let config = parse_sections(&config_text(), ConfigFileKind::Config);
        assert_eq!(config.names(), ["default", "prod", "dev"]);
        assert_eq!(config.region_of("default"), Some("us-west-2"));
        assert_eq!(config.region_of("prod"), Some("ap-northeast-1"));
        assert_eq!(config.region_of("dev"), None);
        assert_eq!(
            config.region_of("corp"),
            None,
            "sso-session is not a profile"
        );
    }

    #[test]
    fn credentials_sections_give_names_but_no_regions() {
        let credentials = parse_sections(&credentials_text(), ConfigFileKind::Credentials);
        assert_eq!(credentials.names(), ["dev", "ci"]);
        assert_eq!(credentials.region_of("dev"), None);
    }

    #[test]
    fn profile_list_is_sdk_default_then_unique_sorted_names() {
        let config = parse_sections(&config_text(), ConfigFileKind::Config);
        let credentials = parse_sections(&credentials_text(), ConfigFileKind::Credentials);
        let profiles = build_profiles(&config, &credentials, &EnvRegionHints::default());
        assert_eq!(
            profiles,
            vec![
                sdk_default(Some("us-west-2")),
                named("ci", None),
                named("default", Some("us-west-2")),
                named("dev", None),
                named("prod", Some("ap-northeast-1")),
            ]
        );
    }

    #[test]
    fn credential_values_are_never_kept() {
        let config = parse_sections(&config_text(), ConfigFileKind::Config);
        let credentials = parse_sections(&credentials_text(), ConfigFileKind::Credentials);
        let profiles = build_profiles(&config, &credentials, &EnvRegionHints::default());
        let everything = format!("{config:?} {credentials:?} {profiles:?}");
        assert!(!everything.contains(&dummy_secret()));
        assert!(!everything.contains(&dummy_key_id()));
        assert!(!everything.contains("example.invalid"));
        assert!(!everything.contains("json"));
    }

    #[test]
    fn sdk_default_region_follows_the_sdk_order() {
        let config = parse_sections(&config_text(), ConfigFileKind::Config);
        let none = ProfileSections::default();
        let region = |env: EnvRegionHints| {
            build_profiles(&config, &none, &env)[0]
                .default_region
                .clone()
        };

        let all = EnvRegionHints {
            aws_region: Some("eu-central-1".to_string()),
            aws_default_region: Some("eu-north-1".to_string()),
            aws_profile: Some("prod".to_string()),
        };
        assert_eq!(region(all.clone()), Some("eu-central-1".to_string()));
        let no_region = EnvRegionHints {
            aws_region: None,
            ..all.clone()
        };
        assert_eq!(region(no_region.clone()), Some("eu-north-1".to_string()));
        let profile_only = EnvRegionHints {
            aws_default_region: None,
            ..no_region
        };
        assert_eq!(region(profile_only), Some("ap-northeast-1".to_string()));
        let profile_without_region = EnvRegionHints {
            aws_profile: Some("dev".to_string()),
            ..EnvRegionHints::default()
        };
        assert_eq!(region(profile_without_region), None);
        assert_eq!(
            region(EnvRegionHints::default()),
            Some("us-west-2".to_string())
        );
        let blank = EnvRegionHints {
            aws_region: Some("  ".to_string()),
            ..EnvRegionHints::default()
        };
        assert_eq!(region(blank), Some("us-west-2".to_string()));
    }

    #[test]
    fn broken_lines_are_skipped_and_the_rest_is_used() {
        let text = "[profile ok]\nregion = us-east-1\n[broken\nregion = eu-west-1\nnot a key value\n\
                    [profile also-ok]\nregion = us-east-2\n";
        let config = parse_sections(text, ConfigFileKind::Config);
        assert_eq!(config.names(), ["ok", "also-ok"]);
        assert_eq!(config.region_of("ok"), Some("us-east-1"));
        assert_eq!(config.region_of("also-ok"), Some("us-east-2"));
    }

    /// R-01 (human decision b): an unclosed `[profile x` header is not an
    /// unreadable file; the line is skipped, its `region` is not attached to
    /// any profile, and every readable section is still used.
    #[test]
    fn unclosed_profile_header_is_skipped_and_the_rest_is_used() {
        let text = "[profile before]\nregion = us-east-1\n[profile x\nregion = eu-west-1\n\
                    [profile after]\nregion = ap-south-1\n";
        let config = parse_sections(text, ConfigFileKind::Config);
        assert_eq!(config.names(), ["before", "after"]);
        assert_eq!(config.region_of("before"), Some("us-east-1"));
        assert_eq!(config.region_of("x"), None);
        assert_eq!(config.region_of("after"), Some("ap-south-1"));
        let profiles = build_profiles(
            &config,
            &ProfileSections::default(),
            &EnvRegionHints::default(),
        );
        assert_eq!(
            profiles.len(),
            3,
            "SDK default plus the two readable profiles"
        );
    }

    #[test]
    fn empty_files_give_only_the_sdk_default() {
        let none = ProfileSections::default();
        assert_eq!(
            build_profiles(&none, &none, &EnvRegionHints::default()),
            vec![sdk_default(None)]
        );
    }

    #[test]
    fn selectors_and_catalog_lookups() {
        assert_eq!(ProfileSelector::SdkDefault.profile_name(), None);
        assert_eq!(
            ProfileSelector::Named("dev".to_string()).profile_name(),
            Some("dev")
        );
        assert_eq!(
            ProfileSelector::from_optional_name(None),
            ProfileSelector::SdkDefault
        );
        assert_eq!(
            ProfileSelector::from_optional_name(Some("  ")),
            ProfileSelector::SdkDefault
        );
        assert_eq!(
            ProfileSelector::from_optional_name(Some(" dev ")),
            ProfileSelector::Named("dev".to_string())
        );
        let catalog = ConnectionCatalog::new(
            vec![sdk_default(None), named("dev", Some("xx-test-1"))],
            vec![ConfigFileKind::Credentials],
        );
        assert_eq!(
            catalog.profiles()[1].selector(),
            ProfileSelector::Named("dev".to_string())
        );
        assert!(
            catalog
                .find(&ProfileSelector::Named("dev".to_string()))
                .is_some()
        );
        assert!(
            catalog
                .find(&ProfileSelector::Named("nope".to_string()))
                .is_none()
        );
        assert!(catalog.has_region("xx-test-1"));
        assert!(catalog.has_region("us-east-1"));
        assert!(!catalog.has_region("moon-1"));
        assert_eq!(catalog.unreadable_files(), [ConfigFileKind::Credentials]);
        assert_eq!(
            ConfigFileKind::Credentials.unreadable_message_key(),
            "catalog.unreadable.credentials"
        );
        assert_eq!(
            ConfigFileKind::Config.unreadable_message_key(),
            "catalog.unreadable.config"
        );
    }

    // --- 261011-sso-login: SSO settings and source_profile (BR1.1, BR1.2) ---

    fn sso_config_text() -> String {
        "[default]\nsso_session = corp\nregion = us-east-1\n\
         [profile dev]\nsso_start_url = https://x.awsapps.com/start\nsso_account_id = 111122223333\n\
         [profile role]\nsource_profile = dev\nrole_arn = arn:aws:iam::111122223333:role/R\n\
         [profile keys]\nregion = eu-west-1\n\
         [sso-session corp]\nsso_start_url = https://corp.awsapps.com/start\nsso_region = us-east-1\n"
            .to_string()
    }

    #[test]
    fn sso_session_key_marks_a_profile_as_sso() {
        let config = parse_sections(&sso_config_text(), ConfigFileKind::Config);
        assert!(config.has_sso_settings("default"));
    }

    #[test]
    fn sso_start_url_key_marks_a_profile_as_sso() {
        let config = parse_sections(&sso_config_text(), ConfigFileKind::Config);
        assert!(config.has_sso_settings("dev"));
    }

    #[test]
    fn a_profile_without_sso_keys_is_not_sso() {
        let config = parse_sections(&sso_config_text(), ConfigFileKind::Config);
        assert!(!config.has_sso_settings("role"));
        assert!(!config.has_sso_settings("keys"));
        assert!(!config.has_sso_settings("no-such-profile"));
        assert!(
            !config.has_sso_settings("corp"),
            "sso-session is not a profile"
        );
        assert_eq!(config.names(), ["default", "dev", "role", "keys"]);
    }

    #[test]
    fn source_profile_keeps_only_the_name() {
        let config = parse_sections(&sso_config_text(), ConfigFileKind::Config);
        assert_eq!(config.source_profile_of("role"), Some("dev"));
        assert_eq!(config.source_profile_of("dev"), None);
        assert_eq!(
            config.auth_infos(),
            vec![
                ProfileAuthInfo {
                    profile_name: "default".to_string(),
                    has_sso_settings: true,
                    source_profile: None,
                },
                ProfileAuthInfo {
                    profile_name: "dev".to_string(),
                    has_sso_settings: true,
                    source_profile: None,
                },
                ProfileAuthInfo {
                    profile_name: "keys".to_string(),
                    has_sso_settings: false,
                    source_profile: None,
                },
                ProfileAuthInfo {
                    profile_name: "role".to_string(),
                    has_sso_settings: false,
                    source_profile: Some("dev".to_string()),
                },
            ]
        );
    }

    #[test]
    fn sso_values_and_credentials_file_lines_are_never_kept() {
        let config = parse_sections(&sso_config_text(), ConfigFileKind::Config);
        let credentials_text = format!(
            "[dev]\naws_access_key_id = {}\nsso_session = from-credentials\n\
             source_profile = from-credentials\n",
            dummy_key_id()
        );
        let credentials = parse_sections(&credentials_text, ConfigFileKind::Credentials);
        assert!(!credentials.has_sso_settings("dev"));
        assert_eq!(credentials.source_profile_of("dev"), None);
        let everything = format!("{config:?} {credentials:?} {:?}", config.auth_infos());
        for value in [
            "awsapps.com",
            "111122223333",
            "arn:aws",
            "corp",
            "from-credentials",
            dummy_key_id().as_str(),
        ] {
            assert!(!everything.contains(value), "{value} must not be kept");
        }
    }

    #[test]
    fn sso_keys_are_read_with_spaces_and_comments_around_them() {
        let text = "# leading comment\n[profile a]\n  sso_session=x  \n; comment\n\
                    [profile b]\n# sso_session = commented-out\n\
                    [profile c]\n   source_profile   =   a   \n\
                    [profile d]\nsso_session_extra = not-the-key\n";
        let config = parse_sections(text, ConfigFileKind::Config);
        assert!(config.has_sso_settings("a"));
        assert!(!config.has_sso_settings("b"));
        assert_eq!(config.source_profile_of("c"), Some("a"));
        assert!(!config.has_sso_settings("d"));
    }

    #[test]
    fn the_catalog_answers_the_auth_info_of_a_profile() {
        let config = parse_sections(&sso_config_text(), ConfigFileKind::Config);
        let catalog = ConnectionCatalog::new(vec![sdk_default(None)], Vec::new())
            .with_auth_infos(config.auth_infos());
        assert!(
            catalog
                .auth_info("dev")
                .is_some_and(|info| info.has_sso_settings)
        );
        assert_eq!(
            catalog
                .auth_info("role")
                .and_then(|info| info.source_profile.as_deref()),
            Some("dev")
        );
        assert!(catalog.auth_info("nope").is_none());
    }
}
