use ethers_core::types::Bytes;
use serde::{Deserialize, Serialize};
use serde_with::formats::PreferOne;
use serde_with::{serde_as, OneOrMany};

use crate::{BlockNumber, H160, H256, U256, U64};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum BlockFilter {
    #[serde(rename_all = "camelCase")]
    Exact { block_hash: H256 },
    #[serde(rename_all = "camelCase")]
    Bounded {
        from_block: Option<BlockNumber>,
        to_block: Option<BlockNumber>,
    },
}

#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LogAddressFilter(#[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")] pub Vec<H160>);

#[serde_as]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LogTopicFilter(#[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")] pub Vec<H256>);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogFilter {
    #[serde(flatten)]
    pub block_filter: Option<BlockFilter>,
    pub address: Option<LogAddressFilter>,
    pub topics: Option<Vec<Option<LogTopicFilter>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Transaction's log entry.
pub struct TransactionLog {
    /// Log's index within transaction.
    pub log_index: U256,
    /// Transaction's index within block.
    pub transaction_index: U64,
    /// Transaction's hash.
    pub transaction_hash: H256,
    /// Block's hash, transaction is included in.
    pub block_hash: H256,
    /// Block number, transaction is included in.
    pub block_number: U64,
    /// Log's address.
    pub address: H160,
    /// Log's data.
    pub data: Bytes,
    /// Log's Topics.
    pub topics: Vec<H256>,
}
