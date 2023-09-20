use serde::{Deserialize, Serialize};

use crate::U256;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeHistory {
    /// An array of block base fees per gas.
    pub base_fee_per_gas: Vec<U256>,
    /// An array of block gas used ratios.
    /// These are calculated as the ratio of `gas_used` and `gas_limit`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub gas_used_ratio: Vec<f64>,
    /// Lowest number block of the returned range.
    pub oldest_block: U256,
    /// An (optional) array of effective priority fee per gas data points from a single
    /// block. All zeroes are returned if the block is empty.
    #[serde(default)]
    pub reward: Option<Vec<Vec<U256>>>,
}
