//! "Is there a newer Portmanteau?" — the one place the program talks to the
//! network. It asks GitHub for the latest published release of this project and
//! compares version numbers. It sends nothing about the writer or their
//! projects (only a `User-Agent` naming this program and its version), never
//! runs unless the writer lets it (the app checks once a day unless switched
//! off, and when asked from the Help menu), and never installs anything by
//! itself.
//!
//! The decision logic is pure ([`decide`], [`parse_release`], [`is_due`]) so it
//! is tested from saved JSON; [`check`] is the single function that touches the
//! network.

pub mod install;

use std::time::Duration;

pub use semver::Version;
use serde::Deserialize;

/// The releases of this project, newest published (never a draft or a
/// pre-release) first.
pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/davidroberthoare/portmanteau/releases/latest";

/// How long to wait between automatic checks.
pub const CHECK_INTERVAL_SECS: i64 = 24 * 60 * 60;

/// The version of the running program.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One downloadable file attached to a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A published release, as much as the app needs to describe it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub tag: String,
    /// The release's page on GitHub.
    pub page_url: String,
    /// The release notes, as written (Markdown).
    pub notes: String,
    pub assets: Vec<Asset>,
}

/// What a check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The running version is the newest (or newer, as in a development build).
    UpToDate { latest: Version },
    Available(Release),
    /// The project has not published a release yet (GitHub answers 404).
    NoReleasesYet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckError {
    /// Offline, timed out, or GitHub did not answer.
    Network(String),
    /// GitHub answered with something that is not a release this program understands.
    BadResponse(String),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Network(why) => write!(f, "Could not reach GitHub ({why})"),
            CheckError::BadResponse(why) => write!(f, "GitHub's answer was not understood ({why})"),
        }
    }
}

impl std::error::Error for CheckError {}

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Deserialize)]
struct RawAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

/// Read a release tag such as `v1.2.3` (or `1.2.3`) as a version number.
pub fn parse_version(tag: &str) -> Option<Version> {
    Version::parse(tag.trim().trim_start_matches('v')).ok()
}

/// Understand GitHub's "latest release" JSON. Drafts and pre-releases are
/// refused: they are never offered as updates.
pub fn parse_release(json: &str) -> Result<Release, CheckError> {
    let raw: RawRelease =
        serde_json::from_str(json).map_err(|e| CheckError::BadResponse(e.to_string()))?;
    if raw.draft || raw.prerelease {
        return Err(CheckError::BadResponse("not a stable release".into()));
    }
    let version = parse_version(&raw.tag_name)
        .ok_or_else(|| CheckError::BadResponse(format!("tag \"{}\" is not a version", raw.tag_name)))?;
    if !version.pre.is_empty() {
        return Err(CheckError::BadResponse("not a stable release".into()));
    }
    Ok(Release {
        version,
        tag: raw.tag_name,
        page_url: raw.html_url,
        notes: raw.body.unwrap_or_default(),
        assets: raw
            .assets
            .into_iter()
            .map(|a| Asset { name: a.name, url: a.browser_download_url, size: a.size })
            .collect(),
    })
}

/// Compare the running version with GitHub's answer.
pub fn decide(current: &str, latest_json: &str) -> Result<Outcome, CheckError> {
    let current = Version::parse(current)
        .map_err(|e| CheckError::BadResponse(format!("this program's version: {e}")))?;
    let release = parse_release(latest_json)?;
    Ok(if release.version > current {
        Outcome::Available(release)
    } else {
        Outcome::UpToDate { latest: release.version }
    })
}

/// Is an automatic check due? `last_checked` and `now` are Unix seconds. A
/// clock that went backwards counts as due, so a wrong clock cannot stop
/// checks for good.
pub fn is_due(last_checked: Option<i64>, now: i64) -> bool {
    match last_checked {
        None => true,
        Some(last) => now < last || now - last >= CHECK_INTERVAL_SECS,
    }
}

/// Is a release one the writer chose to skip?
pub fn is_skipped(release: &Release, skipped: Option<&str>) -> bool {
    skipped.is_some_and(|s| parse_version(s).is_some_and(|s| s == release.version))
}

/// Ask GitHub for the latest release and compare it with `current`. Blocks for
/// up to ten seconds: call it from a background thread, never the UI thread.
pub fn check(current: &str) -> Result<Outcome, CheckError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let response = match agent
        .get(LATEST_RELEASE_URL)
        .header("User-Agent", concat!("portmanteau/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(Outcome::NoReleasesYet),
        Err(e) => return Err(CheckError::Network(e.to_string())),
    };
    let body = response
        .into_body()
        .read_to_string()
        .map_err(|e| CheckError::Network(e.to_string()))?;
    decide(current, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_json(tag: &str, extra: &str) -> String {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/x/y/releases/tag/{tag}","body":"What's new\n- things","assets":[{{"name":"portmanteau-linux-x86_64.tar.gz","browser_download_url":"https://example.test/a.tar.gz","size":1234}}]{extra}}}"#
        )
    }

    #[test]
    fn versions_are_read_from_tags() {
        assert_eq!(parse_version("v0.2.0"), Some(Version::new(0, 2, 0)));
        assert_eq!(parse_version("1.10.3"), Some(Version::new(1, 10, 3)));
        assert_eq!(parse_version("latest"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn ten_is_newer_than_nine() {
        let out = decide("0.9.0", &release_json("v0.10.0", "")).unwrap();
        assert!(matches!(out, Outcome::Available(r) if r.version == Version::new(0, 10, 0)));
    }

    #[test]
    fn same_or_older_release_is_up_to_date() {
        for current in ["0.2.0", "0.3.0", "1.0.0"] {
            let out = decide(current, &release_json("v0.2.0", "")).unwrap();
            assert!(matches!(out, Outcome::UpToDate { .. }), "{current}");
        }
    }

    #[test]
    fn a_release_carries_its_notes_page_and_files() {
        let Outcome::Available(r) = decide("0.1.0", &release_json("v0.2.0", "")).unwrap() else {
            panic!("expected an update")
        };
        assert_eq!(r.tag, "v0.2.0");
        assert!(r.page_url.ends_with("/v0.2.0"));
        assert!(r.notes.starts_with("What's new"));
        assert_eq!(r.assets.len(), 1);
        assert_eq!(r.assets[0].size, 1234);
    }

    #[test]
    fn drafts_and_prereleases_are_never_offered() {
        for flag in [r#","prerelease":true"#, r#","draft":true"#] {
            assert!(decide("0.1.0", &release_json("v9.0.0", flag)).is_err(), "{flag}");
        }
        assert!(decide("0.1.0", &release_json("v9.0.0-beta.1", "")).is_err());
    }

    #[test]
    fn nonsense_is_an_error_not_a_panic() {
        assert!(matches!(decide("0.1.0", "<html>rate limited</html>"), Err(CheckError::BadResponse(_))));
        assert!(matches!(decide("0.1.0", r#"{"message":"Not Found"}"#), Err(CheckError::BadResponse(_))));
        assert!(matches!(decide("0.1.0", &release_json("nightly", "")), Err(CheckError::BadResponse(_))));
        assert!(decide("not a version", &release_json("v0.2.0", "")).is_err());
    }

    #[test]
    fn automatic_checks_run_at_most_once_a_day() {
        let now = 1_800_000_000;
        assert!(is_due(None, now));
        assert!(!is_due(Some(now - 60), now));
        assert!(!is_due(Some(now - CHECK_INTERVAL_SECS + 1), now));
        assert!(is_due(Some(now - CHECK_INTERVAL_SECS), now));
        assert!(is_due(Some(now + 5_000), now), "a clock set back must not stop checks");
    }

    #[test]
    fn a_skipped_version_is_not_offered_but_a_newer_one_is() {
        let Outcome::Available(r) = decide("0.1.0", &release_json("v0.2.0", "")).unwrap() else { panic!() };
        assert!(is_skipped(&r, Some("0.2.0")));
        assert!(is_skipped(&r, Some("v0.2.0")));
        assert!(!is_skipped(&r, Some("0.1.5")));
        assert!(!is_skipped(&r, None));
    }

    #[test]
    fn this_programs_own_version_is_valid() {
        assert!(Version::parse(CURRENT_VERSION).is_ok());
    }
}
