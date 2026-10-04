//! Region options (BR1.4): a static, built-in list of the public regions of
//! the standard AWS partition. No network, no API call (the Rust SDK does
//! not expose its region list, so the rule's built-in list is used).

use super::ConnectionProfile;

/// Public regions of the standard partition, ascending.
pub const STANDARD_REGIONS: &[&str] = &[
    "af-south-1",
    "ap-east-1",
    "ap-east-2",
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-northeast-3",
    "ap-south-1",
    "ap-south-2",
    "ap-southeast-1",
    "ap-southeast-2",
    "ap-southeast-3",
    "ap-southeast-4",
    "ap-southeast-5",
    "ap-southeast-6",
    "ap-southeast-7",
    "ca-central-1",
    "ca-west-1",
    "eu-central-1",
    "eu-central-2",
    "eu-north-1",
    "eu-south-1",
    "eu-south-2",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "il-central-1",
    "me-central-1",
    "me-south-1",
    "mx-central-1",
    "sa-east-1",
    "us-east-1",
    "us-east-2",
    "us-west-1",
    "us-west-2",
];

/// Region options: the built-in list plus any profile default region that
/// is missing from it (BR1.5), sorted by code and without duplicates.
pub fn region_options(profiles: &[ConnectionProfile]) -> Vec<String> {
    let mut options: Vec<String> = STANDARD_REGIONS
        .iter()
        .map(|code| code.to_string())
        .chain(profiles.iter().filter_map(|p| p.default_region.clone()))
        .collect();
    options.sort();
    options.dedup();
    options
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::ProfileKind;

    fn profile(region: Option<&str>) -> ConnectionProfile {
        ConnectionProfile {
            kind: ProfileKind::Named,
            profile_name: Some("p".to_string()),
            default_region: region.map(str::to_string),
        }
    }

    #[test]
    fn built_in_list_is_non_empty_sorted_and_unique() {
        assert!(STANDARD_REGIONS.len() >= 30);
        assert!(STANDARD_REGIONS.windows(2).all(|pair| pair[0] < pair[1]));
        for code in ["us-east-1", "ap-northeast-1", "eu-west-1"] {
            assert!(STANDARD_REGIONS.contains(&code), "{code}");
        }
    }

    #[test]
    fn options_are_the_built_in_list_when_defaults_are_known() {
        let options = region_options(&[profile(Some("ap-northeast-1")), profile(None)]);
        let expected: Vec<String> = STANDARD_REGIONS.iter().map(|c| c.to_string()).collect();
        assert_eq!(options, expected);
    }

    #[test]
    fn unknown_default_regions_are_added_in_order() {
        let options = region_options(&[profile(Some("zz-local-1")), profile(Some("aa-test-1"))]);
        assert_eq!(options.len(), STANDARD_REGIONS.len() + 2);
        assert_eq!(options.first().map(String::as_str), Some("aa-test-1"));
        assert_eq!(options.last().map(String::as_str), Some("zz-local-1"));
        assert!(options.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
