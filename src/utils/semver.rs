use lazy_static::lazy_static;
use regex::Regex;
use semver::{Version, VersionReq};

use crate::utils::errors::{ResultTrait as _, ResultWithError};

lazy_static! {
    static ref VERSION_RE: Regex = Regex::new(r"\d+(\.\d+)+").unwrap();
}

pub struct SemverUtils {}

impl SemverUtils {
    pub fn is_version_greater_or_equal(min_version: &str, input: &str) -> ResultWithError<bool> {
        if let Some(capt) = VERSION_RE.find(input) {
            let found_version_str = capt.as_str();
            let found_version = Version::parse(found_version_str)
                .auto_err(&format!("Failed to parse version: {}", found_version_str))?;

            let required_version = VersionReq::parse(min_version)
                .auto_err(&format!("Invalid min_version in config: {}", min_version))?;

            return if required_version.matches(&found_version) {
                Ok(true)
            } else {
                Ok(false)
            };
        }

        Ok(false)
    }

    pub fn is_valid_version(input: &str) -> bool {
        if let Some(capt) = VERSION_RE.find(input) {
            let found_version_str = capt.as_str();
            return Version::parse(found_version_str).is_ok();
        }
        false
    }

    pub fn extract_version(name: &str) -> Option<Version> {
        let re = Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
        let caps = re.captures(name)?;
        Version::parse(&caps[1]).ok()
    }
}
