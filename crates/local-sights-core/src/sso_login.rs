//! SSO login through the AWS CLI (261011-sso-login): the pure decisions.
//!
//! The app never signs in by itself: it starts `aws sso login` of the AWS
//! CLI, which opens the browser, calls the SSO APIs and stores the token.
//! This module decides, without touching the process, the files or AWS:
//!
//! - which profile to sign in with ([`resolve_login_target`], BR1.3, BR1.4,
//!   BR2.3): the selected profile, or the one its `source_profile` chain
//!   leads to that has SSO settings;
//! - the exact CLI arguments ([`cli_args`], BR2.2): `sso login` and, when
//!   needed, `--profile <name>` with a name from the profile list;
//! - what of a CLI output line may be shown ([`extract_login_hints`],
//!   BR3.1): only an allow-listed `https://` URL and a verification code;
//!   every other text is dropped.
//!
//! Nothing here calls an SSO API or reads the SSO token cache (BR4.3).

use std::sync::LazyLock;

use regex::Regex;

use crate::catalog::{ConnectionCatalog, ProfileSelector};

/// Most `source_profile` steps followed from the selected profile (BR1.4).
pub const MAX_SOURCE_PROFILE_HOPS: usize = 5;
/// Bytes of one CLI output line that are looked at; the rest is dropped.
pub const MAX_LINE_BYTES: usize = 4096;
/// Longest URL that is shown.
const MAX_URL_LEN: usize = 2048;
/// Host suffixes of the sign-in pages the CLI prints (BR3.1).
const ALLOWED_HOST_SUFFIXES: [&str; 2] = [".amazonaws.com", ".awsapps.com"];

/// A verification code: four upper-case letters, a hyphen, four more.
static USER_CODE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{4}-[A-Z]{4}$").ok());

/// Where `aws sso login` signs in (entities.md SsoLoginTarget).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsoLoginTarget {
    /// The profile the selection stands for: the named profile, or for the
    /// SDK default `AWS_PROFILE`, otherwise `default` (BR1.3).
    pub effective_profile_name: String,
    /// The profile with the SSO settings, reached through `source_profile`
    /// (BR1.4).
    pub login_profile_name: String,
    /// The name passed with `--profile`; `None` when the CLI picks the same
    /// profile by itself (BR2.3).
    pub profile_argument: Option<String>,
}

/// What one CLI output line may show (BR3.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginHints {
    /// The first allow-listed `https://` URL of the line.
    pub verification_url: Option<String>,
    /// The first verification code of the line.
    pub user_code: Option<String>,
}

impl LoginHints {
    /// Whether the line gave nothing to show.
    pub fn is_empty(&self) -> bool {
        self.verification_url.is_none() && self.user_code.is_none()
    }
}

/// How one login attempt ended, as the app reports it to the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsoLoginOutcome {
    /// The CLI ended with exit code 0.
    Succeeded,
    /// The CLI ended with another code, or could not be started (`None`).
    Failed {
        /// The exit code, when the CLI ran and ended with one.
        exit_code: Option<i32>,
    },
    /// Five minutes passed; the CLI was stopped (BR3.2).
    TimedOut,
    /// The user cancelled; the CLI was stopped (BR3.3).
    Cancelled,
    /// No AWS CLI was found (BR2.1).
    CliNotFound,
}

/// Decides where to sign in (BR1.3, BR1.4, BR2.3), or `None` when there is
/// no SSO profile to sign in with: the profile is not in the list, has no
/// SSO settings and no usable `source_profile`, the chain loops, leaves the
/// list or is longer than [`MAX_SOURCE_PROFILE_HOPS`].
pub fn resolve_login_target(
    selector: &ProfileSelector,
    catalog: &ConnectionCatalog,
    aws_profile_env: Option<&str>,
) -> Option<SsoLoginTarget> {
    let effective = effective_profile_name(selector, aws_profile_env);
    let login = follow_source_profiles(&effective, catalog)?;
    let profile_argument = match selector {
        ProfileSelector::SdkDefault if login == effective => None,
        _ => Some(login.clone()),
    };
    Some(SsoLoginTarget {
        effective_profile_name: effective,
        login_profile_name: login,
        profile_argument,
    })
}

/// BR1.3: the named profile, or for the SDK default a non-blank
/// `AWS_PROFILE`, otherwise `default`.
fn effective_profile_name(selector: &ProfileSelector, aws_profile_env: Option<&str>) -> String {
    match selector {
        ProfileSelector::Named(name) => name.clone(),
        ProfileSelector::SdkDefault => aws_profile_env
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("default")
            .to_string(),
    }
}

/// BR1.4: from `start`, follows `source_profile` until a profile with SSO
/// settings, at most [`MAX_SOURCE_PROFILE_HOPS`] steps. Every profile on
/// the way must be in the profile list; a loop ends the search.
fn follow_source_profiles(start: &str, catalog: &ConnectionCatalog) -> Option<String> {
    let mut visited: Vec<&str> = Vec::new();
    let mut current = start;
    for hops in 0..=MAX_SOURCE_PROFILE_HOPS {
        if visited.contains(&current) || !is_listed(current, catalog) {
            return None;
        }
        let info = catalog.auth_info(current)?;
        if info.has_sso_settings {
            return Some(current.to_string());
        }
        if hops == MAX_SOURCE_PROFILE_HOPS {
            return None;
        }
        visited.push(current);
        current = info.source_profile.as_deref()?;
    }
    None
}

/// Whether `name` is a named profile of the list.
fn is_listed(name: &str, catalog: &ConnectionCatalog) -> bool {
    catalog
        .find(&ProfileSelector::Named(name.to_string()))
        .is_some()
}

/// The CLI arguments (BR2.2): `sso login`, plus `--profile <name>` when the
/// target needs it. `None` when that name is not in the profile list, so
/// nothing is started.
pub fn cli_args(target: &SsoLoginTarget, catalog: &ConnectionCatalog) -> Option<Vec<String>> {
    let mut args = vec!["sso".to_string(), "login".to_string()];
    if let Some(name) = &target.profile_argument {
        if !is_listed(name, catalog) {
            return None;
        }
        args.push("--profile".to_string());
        args.push(name.clone());
    }
    Some(args)
}

/// The allow-listed URL and the verification code of one CLI output line
/// (BR3.1). Only the first [`MAX_LINE_BYTES`] bytes are looked at.
pub fn extract_login_hints(line: &str) -> LoginHints {
    let mut hints = LoginHints::default();
    for word in truncate_to_bytes(line, MAX_LINE_BYTES).split_whitespace() {
        let word = trim_punctuation(word);
        if hints.verification_url.is_none() && is_allowed_login_url(word) {
            hints.verification_url = Some(word.to_string());
        } else if hints.user_code.is_none() && is_user_code(word) {
            hints.user_code = Some(word.to_string());
        }
    }
    hints
}

/// The longest prefix of `text` of at most `max` bytes ending on a
/// character boundary.
fn truncate_to_bytes(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Strips quotes and brackets around a word and sentence punctuation after
/// it, so `"https://.../",` or `(ABCD-EFGH).` are recognised.
fn trim_punctuation(word: &str) -> &str {
    word.trim_start_matches(['"', '\'', '(', '<', '['])
        .trim_end_matches(['"', '\'', ')', '>', ']', ',', '.', ';', ':'])
}

/// Whether `word` is a verification code (four upper-case letters, a
/// hyphen, four more).
fn is_user_code(word: &str) -> bool {
    USER_CODE
        .as_ref()
        .is_some_and(|pattern| pattern.is_match(word))
}

/// Whether `url` is a sign-in page the app may show and open (BR3.1): an
/// `https://` URL without user info or port whose host ends with
/// `.amazonaws.com` or `.awsapps.com`.
pub fn is_allowed_login_url(url: &str) -> bool {
    if url.len() > MAX_URL_LEN || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..host_end].to_ascii_lowercase();
    let well_formed = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    well_formed
        && ALLOWED_HOST_SUFFIXES
            .iter()
            .any(|suffix| host.len() > suffix.len() && host.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{
        ConfigFileKind, ConnectionProfile, EnvRegionHints, ProfileSections, build_profiles,
        parse_sections,
    };

    fn catalog_from(config_text: &str) -> ConnectionCatalog {
        let config = parse_sections(config_text, ConfigFileKind::Config);
        let profiles: Vec<ConnectionProfile> = build_profiles(
            &config,
            &ProfileSections::default(),
            &EnvRegionHints::default(),
        );
        ConnectionCatalog::new(profiles, Vec::new()).with_auth_infos(config.auth_infos())
    }

    fn standard_catalog() -> ConnectionCatalog {
        catalog_from(
            "[default]\nsso_session = corp\n\
             [profile dev]\nsso_start_url = https://x.awsapps.com/start\n\
             [profile role]\nsource_profile = dev\nrole_arn = arn:aws:iam::111122223333:role/R\n\
             [profile keys]\nregion = eu-west-1\n\
             [profile ghost-role]\nsource_profile = ghost\n\
             [sso-session corp]\nsso_region = us-east-1\n",
        )
    }

    fn named(name: &str) -> ProfileSelector {
        ProfileSelector::Named(name.to_string())
    }

    fn target(effective: &str, login: &str, argument: Option<&str>) -> SsoLoginTarget {
        SsoLoginTarget {
            effective_profile_name: effective.to_string(),
            login_profile_name: login.to_string(),
            profile_argument: argument.map(str::to_string),
        }
    }

    /// `[default] -> p1 -> ... -> pN`, where only pN has SSO settings.
    fn chain_catalog(hops: usize) -> ConnectionCatalog {
        let mut text = String::from("[default]\nsource_profile = p1\n");
        for index in 1..hops {
            text.push_str(&format!(
                "[profile p{index}]\nsource_profile = p{}\n",
                index + 1
            ));
        }
        text.push_str(&format!("[profile p{hops}]\nsso_session = corp\n"));
        catalog_from(&text)
    }

    // --- resolve_login_target (BR1.3, BR1.4, BR2.3) ---

    #[test]
    fn a_named_sso_profile_signs_in_with_profile() {
        assert_eq!(
            resolve_login_target(&named("dev"), &standard_catalog(), None),
            Some(target("dev", "dev", Some("dev")))
        );
    }

    #[test]
    fn the_sdk_default_without_aws_profile_signs_in_default_without_profile() {
        assert_eq!(
            resolve_login_target(&ProfileSelector::SdkDefault, &standard_catalog(), None),
            Some(target("default", "default", None))
        );
    }

    #[test]
    fn the_sdk_default_with_aws_profile_signs_in_that_profile_without_profile() {
        assert_eq!(
            resolve_login_target(
                &ProfileSelector::SdkDefault,
                &standard_catalog(),
                Some("dev")
            ),
            Some(target("dev", "dev", None))
        );
        assert_eq!(
            resolve_login_target(
                &ProfileSelector::SdkDefault,
                &standard_catalog(),
                Some("  ")
            ),
            Some(target("default", "default", None)),
            "a blank AWS_PROFILE counts as unset"
        );
    }

    #[test]
    fn an_assume_role_profile_signs_in_its_source_profile() {
        assert_eq!(
            resolve_login_target(&named("role"), &standard_catalog(), None),
            Some(target("role", "dev", Some("dev")))
        );
    }

    #[test]
    fn the_sdk_default_through_a_source_profile_passes_profile() {
        let catalog =
            catalog_from("[default]\nsource_profile = b\n[profile b]\nsso_session = corp\n");
        assert_eq!(
            resolve_login_target(&ProfileSelector::SdkDefault, &catalog, None),
            Some(target("default", "b", Some("b")))
        );
    }

    #[test]
    fn chains_of_two_and_five_steps_are_followed() {
        assert_eq!(
            resolve_login_target(&ProfileSelector::SdkDefault, &chain_catalog(2), None),
            Some(target("default", "p2", Some("p2")))
        );
        assert_eq!(
            resolve_login_target(&ProfileSelector::SdkDefault, &chain_catalog(5), None),
            Some(target("default", "p5", Some("p5")))
        );
    }

    #[test]
    fn a_chain_of_six_steps_has_no_target() {
        assert_eq!(
            resolve_login_target(&ProfileSelector::SdkDefault, &chain_catalog(6), None),
            None
        );
    }

    #[test]
    fn a_loop_has_no_target() {
        let catalog =
            catalog_from("[profile a]\nsource_profile = b\n[profile b]\nsource_profile = a\n");
        assert_eq!(resolve_login_target(&named("a"), &catalog, None), None);
        let self_loop = catalog_from("[profile s]\nsource_profile = s\n");
        assert_eq!(resolve_login_target(&named("s"), &self_loop, None), None);
    }

    #[test]
    fn a_source_profile_outside_the_list_has_no_target() {
        assert_eq!(
            resolve_login_target(&named("ghost-role"), &standard_catalog(), None),
            None
        );
    }

    #[test]
    fn a_profile_without_sso_has_no_target() {
        let catalog = standard_catalog();
        assert_eq!(resolve_login_target(&named("keys"), &catalog, None), None);
        assert_eq!(
            resolve_login_target(&named("unknown"), &catalog, None),
            None
        );
        assert_eq!(
            resolve_login_target(&ProfileSelector::SdkDefault, &catalog, Some("keys")),
            None
        );
        let no_default = catalog_from("[profile dev]\nsso_session = corp\n");
        assert_eq!(
            resolve_login_target(&ProfileSelector::SdkDefault, &no_default, None),
            None
        );
    }

    // --- cli_args (BR2.2) ---

    #[test]
    fn arguments_without_profile_are_sso_login_only() {
        let catalog = standard_catalog();
        assert_eq!(
            cli_args(&target("default", "default", None), &catalog),
            Some(vec!["sso".to_string(), "login".to_string()])
        );
    }

    #[test]
    fn arguments_with_profile_add_the_listed_name() {
        let catalog = standard_catalog();
        assert_eq!(
            cli_args(&target("role", "dev", Some("dev")), &catalog),
            Some(vec![
                "sso".to_string(),
                "login".to_string(),
                "--profile".to_string(),
                "dev".to_string(),
            ])
        );
    }

    #[test]
    fn no_arguments_for_a_name_outside_the_list() {
        let catalog = standard_catalog();
        assert_eq!(
            cli_args(&target("x", "x", Some("not-listed")), &catalog),
            None
        );
    }

    // --- extract_login_hints, is_allowed_login_url (BR3.1) ---

    fn url_of(line: &str) -> Option<String> {
        extract_login_hints(line).verification_url
    }

    #[test]
    fn device_and_portal_and_authorize_urls_are_taken() {
        assert_eq!(
            url_of("https://device.sso.us-east-1.amazonaws.com/"),
            Some("https://device.sso.us-east-1.amazonaws.com/".to_string())
        );
        assert_eq!(
            url_of("open the following URL: https://my.awsapps.com/start/#/device"),
            Some("https://my.awsapps.com/start/#/device".to_string())
        );
        let authorize =
            "https://oidc.us-east-1.amazonaws.com/authorize?response_type=code&client_id=abc";
        assert_eq!(url_of(authorize), Some(authorize.to_string()));
    }

    #[test]
    fn plain_http_and_foreign_hosts_are_dropped() {
        assert_eq!(url_of("http://device.sso.us-east-1.amazonaws.com/"), None);
        assert_eq!(url_of("https://evil.example.com/"), None);
        assert_eq!(url_of("https://amazonaws.com.evil.example/"), None);
        assert_eq!(
            url_of("https://user@device.sso.us-east-1.amazonaws.com/"),
            None
        );
        assert_eq!(
            url_of("https://device.sso.us-east-1.amazonaws.com:8443/"),
            None
        );
    }

    #[test]
    fn only_the_first_allowed_url_of_a_line_is_taken() {
        assert_eq!(
            url_of(
                "https://evil.example.com/ https://a.awsapps.com/start https://b.awsapps.com/start"
            ),
            Some("https://a.awsapps.com/start".to_string())
        );
    }

    #[test]
    fn a_verification_code_is_taken() {
        assert_eq!(
            extract_login_hints("ABCD-EFGH").user_code,
            Some("ABCD-EFGH".to_string())
        );
        assert_eq!(
            extract_login_hints("Then enter the code: WXYZ-QRST").user_code,
            Some("WXYZ-QRST".to_string())
        );
    }

    #[test]
    fn lower_case_and_wrong_length_codes_are_dropped() {
        for line in [
            "abcd-efgh",
            "ABC-DEFG",
            "ABCDE-FGHI",
            "ABCD-EFG1",
            "ABCD_EFGH",
        ] {
            assert_eq!(extract_login_hints(line).user_code, None, "{line}");
        }
    }

    #[test]
    fn a_line_without_hints_gives_nothing() {
        let hints = extract_login_hints(
            "Attempting to automatically open the SSO authorization page in your default browser.",
        );
        assert!(hints.is_empty());
        assert!(extract_login_hints("").is_empty());
    }

    #[test]
    fn only_the_first_bytes_of_a_long_line_are_looked_at() {
        let mut line = "x".repeat(MAX_LINE_BYTES);
        line.push_str(" https://device.sso.us-east-1.amazonaws.com/ ABCD-EFGH");
        assert!(extract_login_hints(&line).is_empty());
        let mut multibyte = "あ".repeat(MAX_LINE_BYTES);
        multibyte.push_str(" ABCD-EFGH");
        assert!(extract_login_hints(&multibyte).is_empty());
        let mut early = String::from("https://device.sso.us-east-1.amazonaws.com/ ");
        early.push_str(&"y".repeat(MAX_LINE_BYTES * 2));
        assert_eq!(
            extract_login_hints(&early).verification_url,
            Some("https://device.sso.us-east-1.amazonaws.com/".to_string())
        );
    }

    #[test]
    fn the_url_check_stands_alone() {
        assert!(is_allowed_login_url(
            "https://device.sso.us-east-1.amazonaws.com/"
        ));
        assert!(is_allowed_login_url(
            "https://my.awsapps.com/start/#/device"
        ));
        assert!(is_allowed_login_url("https://MY.AWSAPPS.COM/start"));
        assert!(!is_allowed_login_url("https://awsapps.com/start"));
        assert!(!is_allowed_login_url(
            "https://evil.example.com/?x=.amazonaws.com"
        ));
        assert!(!is_allowed_login_url("https://a.awsapps.com/\nnext"));
        assert!(!is_allowed_login_url("file:///etc/passwd"));
        assert!(!is_allowed_login_url(""));
        let long = format!("https://a.awsapps.com/{}", "p".repeat(MAX_URL_LEN));
        assert!(!is_allowed_login_url(&long));
    }
}
