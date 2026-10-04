//! ApiFailure and secret redaction (BR4.3, FR8.3).
//!
//! The safe detail is assembled only from an allow-list of fields (kind, API
//! name, request ID, profile name, role ARN, account ID, log group, log
//! stream). The raw SDK error message is never included. As a second line of
//! defence, user-entered names are masked for access key IDs
//! ([`redact_access_key_ids`]) and every other value goes through the
//! broader [`redact_secrets`].

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

/// Access key IDs: `AKIA`/`ASIA` followed by 16 upper-case letters or digits.
static ACCESS_KEY_ID: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:AKIA|ASIA)[A-Z0-9]{16}").ok());
/// Runs of characters used by base64-style tokens, with optional padding.
static TOKEN_RUN: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9/+_\-]+=*").ok());
/// Runs of characters used by secret access keys.
static SECRET_RUN: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"[A-Za-z0-9/+]+").ok());
/// Length of a secret access key.
const SECRET_ACCESS_KEY_LEN: usize = 40;
/// Minimum length of a run treated as a session or SSO token.
const MIN_TOKEN_LEN: usize = 100;

/// Replacement text for redacted secrets.
pub const REDACTED: &str = "[REDACTED]";

/// Kind of a failed AWS interaction (BR4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum FailureKind {
    /// Credentials are missing or expired (e.g. SSO login required).
    AuthRequired,
    /// The identity lacks permission.
    AccessDenied,
    /// The API throttled the request.
    Throttled,
    /// Connection failure or timeout.
    Network,
    /// The log group or stream does not exist.
    NotFound,
    /// The API rejected a parameter.
    InvalidInput,
    /// No default region was found for the profile (BR1.6).
    RegionMissing,
    /// Anything else.
    Other,
}

/// Allow-listed fields that may appear in a safe detail.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SafeDetailFields {
    /// API operation name, e.g. `GetLogEvents`.
    pub api_name: Option<String>,
    /// AWS request ID.
    pub request_id: Option<String>,
    /// Profile name.
    pub profile_name: Option<String>,
    /// Role ARN.
    pub role_arn: Option<String>,
    /// Account ID.
    pub account_id: Option<String>,
    /// Log group name.
    pub log_group_name: Option<String>,
    /// Log stream name.
    pub log_stream_name: Option<String>,
}

/// A failure converted into a form that is safe to show and log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiFailure {
    kind: FailureKind,
    safe_detail: String,
    retryable: bool,
}

impl FailureKind {
    /// Every variant.
    pub const ALL: [FailureKind; 8] = [
        FailureKind::AuthRequired,
        FailureKind::AccessDenied,
        FailureKind::Throttled,
        FailureKind::Network,
        FailureKind::NotFound,
        FailureKind::InvalidInput,
        FailureKind::RegionMissing,
        FailureKind::Other,
    ];

    /// Stable name, e.g. `"Throttled"`.
    pub fn as_str(self) -> &'static str {
        match self {
            FailureKind::AuthRequired => "AuthRequired",
            FailureKind::AccessDenied => "AccessDenied",
            FailureKind::Throttled => "Throttled",
            FailureKind::Network => "Network",
            FailureKind::NotFound => "NotFound",
            FailureKind::InvalidInput => "InvalidInput",
            FailureKind::RegionMissing => "RegionMissing",
            FailureKind::Other => "Other",
        }
    }

    /// Message-catalog key of the kind name, e.g. `"failure.kind.Throttled"`.
    pub fn message_key(self) -> &'static str {
        match self {
            FailureKind::AuthRequired => "failure.kind.AuthRequired",
            FailureKind::AccessDenied => "failure.kind.AccessDenied",
            FailureKind::Throttled => "failure.kind.Throttled",
            FailureKind::Network => "failure.kind.Network",
            FailureKind::NotFound => "failure.kind.NotFound",
            FailureKind::InvalidInput => "failure.kind.InvalidInput",
            FailureKind::RegionMissing => "failure.kind.RegionMissing",
            FailureKind::Other => "failure.kind.Other",
        }
    }

    /// Only throttling and network failures are worth retrying.
    pub fn is_retryable(self) -> bool {
        matches!(self, FailureKind::Throttled | FailureKind::Network)
    }
}

impl ApiFailure {
    /// Builds a failure whose detail contains only the allow-listed fields.
    pub fn new(kind: FailureKind, fields: &SafeDetailFields) -> Self {
        Self {
            kind,
            safe_detail: build_safe_detail(kind, fields),
            retryable: kind.is_retryable(),
        }
    }

    /// Kind of the failure.
    pub fn kind(&self) -> FailureKind {
        self.kind
    }

    /// Detail that contains no secret and no access key ID.
    pub fn safe_detail(&self) -> &str {
        &self.safe_detail
    }

    /// Whether retrying may help.
    pub fn retryable(&self) -> bool {
        self.retryable
    }
}

/// Builds `kind=...; api=...; ...` from the allow-listed fields only.
///
/// Profile, log group and log stream names (typed by the user) are masked
/// only for access key IDs; the other fields go through [`redact_secrets`].
pub fn build_safe_detail(kind: FailureKind, fields: &SafeDetailFields) -> String {
    // User-entered names get only the access-key-ID redaction, so that long
    // or 40-character names stay readable; every other (SDK-derived) value
    // also gets the secret-key and token heuristics.
    let user_entered: fn(&str) -> String = redact_access_key_ids;
    let sdk_derived: fn(&str) -> String = redact_secrets;
    let labelled = [
        ("api", &fields.api_name, sdk_derived),
        ("requestId", &fields.request_id, sdk_derived),
        ("profile", &fields.profile_name, user_entered),
        ("roleArn", &fields.role_arn, sdk_derived),
        ("accountId", &fields.account_id, sdk_derived),
        ("logGroup", &fields.log_group_name, user_entered),
        ("logStream", &fields.log_stream_name, user_entered),
    ];
    let mut parts = vec![format!("kind={}", kind.as_str())];
    parts.extend(labelled.iter().filter_map(|(label, value, redact)| {
        value
            .as_deref()
            .map(|value| format!("{label}={}", redact(value)))
    }));
    parts.join("; ")
}

/// Masks only access key IDs (`AKIA`/`ASIA` + 16 characters).
///
/// Used for names the user typed (profile, log group, log stream), which may
/// legitimately be long or 40 characters of base64-like text. If the pattern
/// cannot be compiled (a programming error), the whole text is masked.
pub fn redact_access_key_ids(text: &str) -> String {
    match ACCESS_KEY_ID.as_ref() {
        Some(pattern) => pattern.replace_all(text, REDACTED).into_owned(),
        None => REDACTED.to_string(),
    }
}

/// Masks access key IDs (`AKIA`/`ASIA` + 16 characters), 40-character
/// secret-access-key-like strings and long session-token-like strings.
///
/// If a pattern cannot be compiled (which would be a programming error), the
/// whole text is replaced by [`REDACTED`] so that nothing leaks.
pub fn redact_secrets(text: &str) -> String {
    let (Some(access_key_id), Some(token_run), Some(secret_run)) = (
        ACCESS_KEY_ID.as_ref(),
        TOKEN_RUN.as_ref(),
        SECRET_RUN.as_ref(),
    ) else {
        return REDACTED.to_string();
    };
    let text = access_key_id.replace_all(text, REDACTED);
    let text = token_run.replace_all(&text, |caps: &regex::Captures<'_>| {
        let run = &caps[0];
        if run.len() >= MIN_TOKEN_LEN {
            REDACTED.to_string()
        } else {
            secret_run
                .replace_all(run, |inner: &regex::Captures<'_>| {
                    if inner[0].len() == SECRET_ACCESS_KEY_LEN {
                        REDACTED.to_string()
                    } else {
                        inner[0].to_string()
                    }
                })
                .into_owned()
        }
    });
    text.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dummy values are assembled at runtime so that no credential-shaped
    // literal is stored in the repository.
    fn dummy_access_key_id(prefix: &str) -> String {
        format!("{prefix}{}", "EXAMPLE0DUMMY000")
    }

    fn dummy_secret_access_key() -> String {
        format!("{}{}", "Dummy0Secret0Value0For0Tests", "0123456789AB")
    }

    fn dummy_session_token() -> String {
        "IQoJb3JpZ2luX2VjE".repeat(12)
    }

    #[test]
    fn redacts_access_key_ids() {
        for prefix in ["AKIA", "ASIA"] {
            let key = dummy_access_key_id(prefix);
            assert_eq!(key.len(), 20);
            let text = format!("credential {key} rejected");
            assert_eq!(
                redact_secrets(&text),
                format!("credential {REDACTED} rejected")
            );
        }
    }

    #[test]
    fn redacts_secret_access_key_like_strings() {
        let secret = dummy_secret_access_key();
        assert_eq!(secret.len(), 40);
        assert_eq!(
            redact_secrets(&format!("secret={secret}")),
            format!("secret={REDACTED}")
        );
    }

    #[test]
    fn redacts_session_token_like_strings() {
        let token = dummy_session_token();
        assert!(token.len() >= 100);
        assert_eq!(
            redact_secrets(&format!("token {token} end")),
            format!("token {REDACTED} end")
        );
    }

    #[test]
    fn leaves_ordinary_text_unchanged() {
        let text = "profile=dev; roleArn=arn:aws:iam::123456789012:role/ReadLogs; \
                    requestId=0f2c6a52-9d3e-4c1b-8e5a-3f7d2b1c9a40; group=/aws/lambda/orders";
        assert_eq!(redact_secrets(text), text);
    }

    #[test]
    fn safe_detail_contains_only_allow_listed_fields() {
        let fields = SafeDetailFields {
            api_name: Some("GetLogEvents".to_string()),
            request_id: Some("req-1".to_string()),
            profile_name: Some("dev".to_string()),
            role_arn: Some("arn:aws:iam::123456789012:role/ReadLogs".to_string()),
            account_id: Some("123456789012".to_string()),
            log_group_name: Some("/aws/lambda/orders".to_string()),
            log_stream_name: Some("stream-a".to_string()),
        };
        assert_eq!(
            build_safe_detail(FailureKind::AccessDenied, &fields),
            "kind=AccessDenied; api=GetLogEvents; requestId=req-1; profile=dev; \
             roleArn=arn:aws:iam::123456789012:role/ReadLogs; accountId=123456789012; \
             logGroup=/aws/lambda/orders; logStream=stream-a"
        );
    }

    #[test]
    fn safe_detail_omits_absent_fields_and_redacts_values() {
        let fields = SafeDetailFields {
            profile_name: Some(dummy_access_key_id("AKIA")),
            ..SafeDetailFields::default()
        };
        assert_eq!(
            build_safe_detail(FailureKind::AuthRequired, &fields),
            format!("kind=AuthRequired; profile={REDACTED}")
        );
    }

    #[test]
    fn api_failure_carries_kind_detail_and_retryability() {
        let failure = ApiFailure::new(FailureKind::Throttled, &SafeDetailFields::default());
        assert_eq!(failure.kind(), FailureKind::Throttled);
        assert_eq!(failure.safe_detail(), "kind=Throttled");
        assert!(failure.retryable());
        let failure = ApiFailure::new(FailureKind::NotFound, &SafeDetailFields::default());
        assert!(!failure.retryable());
    }

    fn ecs_group_name() -> String {
        // 40 characters of [A-Za-z0-9/]: the shape of a secret access key.
        format!("ecs/app/{}", "0123456789abcdef".repeat(2))
    }

    #[test]
    fn user_entered_names_keep_long_values_visible() {
        let group = ecs_group_name();
        assert_eq!(group.len(), 40);
        let stream = "k8s-node-".repeat(13) + "abc";
        assert_eq!(stream.len(), 120);
        let fields = SafeDetailFields {
            profile_name: Some("p".repeat(110)),
            log_group_name: Some(group.clone()),
            log_stream_name: Some(stream.clone()),
            ..SafeDetailFields::default()
        };
        let detail = build_safe_detail(FailureKind::NotFound, &fields);
        assert!(detail.contains(&format!("logGroup={group}")), "{detail}");
        assert!(detail.contains(&format!("logStream={stream}")), "{detail}");
        assert!(
            detail.contains(&format!("profile={}", "p".repeat(110))),
            "{detail}"
        );
        assert!(!detail.contains(REDACTED), "{detail}");
    }

    #[test]
    fn access_key_id_inside_user_entered_name_is_still_redacted() {
        let fields = SafeDetailFields {
            log_stream_name: Some(format!("app/{}/run", dummy_access_key_id("ASIA"))),
            ..SafeDetailFields::default()
        };
        assert_eq!(
            build_safe_detail(FailureKind::NotFound, &fields),
            format!("kind=NotFound; logStream=app/{REDACTED}/run")
        );
    }

    #[test]
    fn sdk_derived_fields_keep_the_40_and_100_character_heuristics() {
        let fields = SafeDetailFields {
            request_id: Some(dummy_secret_access_key()),
            role_arn: Some(dummy_session_token()),
            ..SafeDetailFields::default()
        };
        assert_eq!(
            build_safe_detail(FailureKind::Other, &fields),
            format!("kind=Other; requestId={REDACTED}; roleArn={REDACTED}")
        );
    }

    #[test]
    fn access_key_id_redaction_leaves_other_long_runs_alone() {
        let group = ecs_group_name();
        assert_eq!(redact_access_key_ids(&group), group);
        let token = dummy_session_token();
        assert_eq!(redact_access_key_ids(&token), token);
        let key = dummy_access_key_id("AKIA");
        assert_eq!(
            redact_access_key_ids(&format!("x {key} y")),
            format!("x {REDACTED} y")
        );
    }

    #[test]
    fn kinds_have_distinct_names_and_message_keys() {
        for kind in FailureKind::ALL {
            assert_eq!(
                kind.message_key(),
                format!("failure.kind.{}", kind.as_str())
            );
        }
        let names: std::collections::HashSet<_> =
            FailureKind::ALL.iter().map(|k| k.as_str()).collect();
        assert_eq!(names.len(), FailureKind::ALL.len());
    }
}
