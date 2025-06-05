use std::borrow::Cow;

use candid::{CandidType, Decode, Encode};
use ic_stable_structures::{Bound, Storable};
use serde::Deserialize;

use crate::build::BuildData;

#[derive(CandidType, Deserialize, Clone, Debug)]
/// Information about a canister upgrade, tracking deployment details and blockchain state.
pub struct UpgradeInfo {
    /// Compilation and build information for the deployed canister version
    pub build_data: BuildData,
    /// Unix timestamp (in seconds) when the upgrade was deployed.
    pub deploy_ts: u64,
    /// The blockchain block number at the time the upgrade was performed.
    pub last_block_number: u64,
}

impl Storable for UpgradeInfo {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::from(Encode!(&self).expect("Failed to encode UpgradeInfo"))
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(&bytes, UpgradeInfo).expect("Failed to decode UpgradeInfo")
    }
}
