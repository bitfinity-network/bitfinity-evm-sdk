use candid::CandidType;
use serde::{Deserialize, Serialize};

// E.g.: x86_64-unknown-linux-gnu
const CARGO_TARGET_TRIPLE: &str = env!("VERGEN_CARGO_TARGET_TRIPLE");
// E.g.: evm
const PKG_NAME: &str = env!("CARGO_PKG_NAME");
// E.g.: 0.1.0
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
// E.g.: 1.64.0
const RUSTC_SEMVER: &str = env!("VERGEN_RUSTC_SEMVER");
// E.g.: 2022-12-23T15:29:20.000000000Z
const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");
// E.g.: true/false
const CARGO_DEBUG: &str = env!("VERGEN_CARGO_DEBUG");
// E.g.: main
const GIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");
// E.g.: acf6c5744b1f4f29c5960a25f4fb4056e2ceedc3
const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
// E.g.: 2022-12-23T15:29:20.000000000Z
const GIT_COMMIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");

/// Contains the build data taken from the environment variables set by the build script.
/// For this to work, the build script must call the `vergen` crate. E.g.:
/// ```rust
/// // Inside build.rs
/// fn main() {
///   vergen::EmitBuilder::builder()
///     .all_build()
///     .all_cargo()
///     .all_git()
///     .all_rustc()
///     .emit()
///     .expect("Cannot set build environment variables");
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct BuildData {
    pub cargo_target_triple: &'static str,
    pub pkg_name: &'static str,
    pub pkg_version: &'static str,
    pub rustc_semver: &'static str,
    pub build_timestamp: &'static str,
    pub cargo_debug: &'static str,
    pub git_branch: &'static str,
    pub git_sha: &'static str,
    pub git_commit_timestamp: &'static str,
}

impl Default for BuildData {
    fn default() -> Self {
        Self {
            cargo_target_triple: CARGO_TARGET_TRIPLE,
            pkg_name: PKG_NAME,
            pkg_version: PKG_VERSION,
            rustc_semver: RUSTC_SEMVER,
            build_timestamp: BUILD_TIMESTAMP,
            cargo_debug: CARGO_DEBUG,
            git_branch: GIT_BRANCH,
            git_sha: GIT_SHA,
            git_commit_timestamp: GIT_COMMIT_TIMESTAMP,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_create_build_data() {
        let build_data = BuildData::default();

        assert_eq!(build_data.cargo_target_triple, CARGO_TARGET_TRIPLE);
        assert_eq!(build_data.pkg_name, PKG_NAME);
        assert_eq!(build_data.pkg_version, PKG_VERSION);
        assert_eq!(build_data.rustc_semver, RUSTC_SEMVER);
        assert_eq!(build_data.build_timestamp, BUILD_TIMESTAMP);
        assert_eq!(build_data.cargo_debug, CARGO_DEBUG);
        assert_eq!(build_data.git_branch, GIT_BRANCH);
        assert_eq!(build_data.git_sha, GIT_SHA);
        assert_eq!(build_data.git_commit_timestamp, GIT_COMMIT_TIMESTAMP);

        assert!(!build_data.cargo_target_triple.is_empty());
        assert!(!build_data.pkg_name.is_empty());
        assert!(!build_data.pkg_version.is_empty());
        assert!(!build_data.rustc_semver.is_empty());
        assert!(!build_data.build_timestamp.is_empty());
        assert!(!build_data.cargo_debug.is_empty());
        assert!(!build_data.git_branch.is_empty());
        assert!(!build_data.git_sha.is_empty());
        assert!(!build_data.git_commit_timestamp.is_empty());

        println!("build data: {:?}", build_data)
    }
}
