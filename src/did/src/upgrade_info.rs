use std::borrow::Cow;

use candid::{CandidType, Decode, Encode};
use ic_stable_structures::{Bound, Storable};
use serde::Deserialize;

use crate::build::BuildData;

/// Historical information about the canister
#[derive(CandidType, Deserialize, Clone, Default, Debug)]
pub struct UpgradeInfo {
    /// The build data of the canister
    pub build_data: BuildData,
    /// The timestamp of the deployment
    pub deploy_ts: u64,
    /// The last block number
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
