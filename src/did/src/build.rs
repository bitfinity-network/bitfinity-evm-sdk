use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Contains the build data.
#[derive(Debug, Clone, Serialize, Deserialize, CandidType, Default)]
pub struct BuildData {
    pub cargo_target_triple: String,
    pub cargo_features: String,
    pub pkg_name: String,
    pub pkg_version: String,
    pub rustc_semver: String,
    pub build_timestamp: String,
    pub cargo_debug: String,
    pub git_branch: String,
    pub git_sha: String,
    pub git_commit_timestamp: String,
}
