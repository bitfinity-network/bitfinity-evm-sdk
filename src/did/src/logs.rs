use candid::CandidType;
use ethers_core::types::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

impl TryFrom<Value> for LogFilter {
    type Error = jsonrpc_core::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Object(ref map) = value {
            // According to documentation if `blockHash` property is specified then `fromBlock` and `toBlock` shouldn't be specified
            if map.contains_key("blockHash")
                && (map.contains_key("fromBlock") || map.contains_key("toBlock"))
            {
                Err(Self::Error::invalid_params(
                    "'blockHash' property cannot be used with 'fromBlock' or 'toBlock'",
                ))
            } else {
                let mut filter: LogFilter =
                    serde_json::from_value(value).map_err(|_| Self::Error::parse_error())?;

                // Empty block filter can be serialized as `block_filter: BlockFilter::Bounded(from_block: None, to_block:None)`
                // That could be OK for us because it is equivalent to `block_filter: None`, but it's better to disambiguate things
                if let Some(BlockFilter::Bounded {
                    from_block: None,
                    to_block: None,
                }) = filter.block_filter
                {
                    filter.block_filter = None;
                }

                Ok(filter)
            }
        } else {
            Err(Self::Error::invalid_params("invalid json value"))
        }
    }
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

/// A log produced by a transaction.
#[derive(Debug, Clone, Default, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub struct Log {
    /// H160. the contract that emitted the log
    pub address: H160,

    /// topics: Array of 0 to 4 32 Bytes of indexed log arguments.
    /// (In solidity: The first topic is the hash of the signature of the event
    /// (e.g. `Deposit(address,bytes32,uint256)`), except you declared the event
    /// with the anonymous specifier.)
    pub topics: Vec<H256>,

    /// Data
    pub data: Vec<u8>,

    /// Block Hash
    #[serde(rename = "blockHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<H256>,

    /// Block Number
    #[serde(rename = "blockNumber")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<U64>,

    /// Transaction Hash
    #[serde(rename = "transactionHash")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_hash: Option<H256>,

    /// Transaction Index
    #[serde(rename = "transactionIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_index: Option<U64>,

    /// Integer of the log index position in the block. None if it's a pending log.
    #[serde(rename = "logIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_index: Option<U256>,

    /// Integer of the transactions index position log was created from.
    /// None when it's a pending log.
    #[serde(rename = "transactionLogIndex")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_log_index: Option<U256>,

    /// Log Type
    #[serde(rename = "logType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_type: Option<String>,

    /// True when the log was removed, due to a chain reorganization.
    /// false if it's a valid log.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<bool>,
}

impl From<ethers_core::types::Log> for Log {
    fn from(log: ethers_core::types::Log) -> Self {
        Self {
            address: log.address.into(),
            topics: log.topics.into_iter().map(Into::into).collect(),
            data: log.data.0.into(),
            block_hash: log.block_hash.map(Into::into),
            block_number: log.block_number.map(Into::into),
            transaction_hash: log.transaction_hash.map(Into::into),
            transaction_index: log.transaction_index.map(Into::into),
            log_index: log.log_index.map(Into::into),
            transaction_log_index: log.transaction_log_index.map(Into::into),
            log_type: log.log_type,
            removed: log.removed,
        }
    }
}

impl From<Log> for ethers_core::types::Log {
    fn from(log: Log) -> Self {
        Self {
            address: log.address.into(),
            topics: log.topics.into_iter().map(Into::into).collect(),
            data: log.data.into(),
            block_hash: log.block_hash.map(Into::into),
            block_number: log.block_number.map(Into::into),
            transaction_hash: log.transaction_hash.map(Into::into),
            transaction_index: log.transaction_index.map(Into::into),
            log_index: log.log_index.map(Into::into),
            transaction_log_index: log.transaction_log_index.map(Into::into),
            log_type: log.log_type,
            removed: log.removed,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const BLOCK_HASH_1: &str = "f43869e67c02c57d1f9a07bb897b54bec1cfa1feb704d91a2ee087566de5df2c";
    const TOPIC_1: &str = "cc6a069bf885d8cf2fb456ca33db48ab7d5e3df1e6504a18e7899a16d604f5c6";
    const TOPIC_2: &str = "e4058f2da8dda0b1ffb454bb7d121c1498dcfc4446a3d86b7c03e27b34e29345";
    const ADDRESS: &str = "7fafd954cbcfd683304cd9be0a85848cbbb1c13d";

    fn get_block_hash_1_str() -> String {
        format!("0x{BLOCK_HASH_1}")
    }

    fn get_block_hash_1() -> H256 {
        H256::from_hex_str(BLOCK_HASH_1).unwrap()
    }

    fn get_topic_1() -> H256 {
        H256::from_hex_str(TOPIC_1).unwrap()
    }

    fn get_topic_1_str() -> String {
        format!("0x{TOPIC_1}")
    }

    fn get_topic_2() -> H256 {
        H256::from_hex_str(TOPIC_2).unwrap()
    }

    fn get_topic_2_str() -> String {
        format!("0x{TOPIC_2}")
    }

    fn get_address_str() -> String {
        format!("0x{ADDRESS}")
    }

    fn get_address_1() -> H160 {
        H160::from_hex_str(ADDRESS).unwrap()
    }

    #[test]
    fn test_log_filter_deserialization_fail() {
        assert!(LogFilter::try_from(json!([])).is_err());
        assert!(LogFilter::try_from(json!("str")).is_err());
        assert!(LogFilter::try_from(json!(42)).is_err());
        assert!(LogFilter::try_from(
            json!({"blockHash": get_block_hash_1_str(), "fromBlock": "earliest"})
        )
        .is_err());
        assert!(LogFilter::try_from(
            json!({"blockHash": get_block_hash_1_str(), "toBlock": "0x01"})
        )
        .is_err());
    }

    #[test]
    fn test_log_filter_deserialization_empty() {
        let filter = LogFilter::try_from(json!({})).unwrap();
        let expected_filter = Default::default();
        assert_eq!(filter, expected_filter);
    }

    #[test]
    fn test_log_filter_deserialization_block_filter() {
        let filter =
            LogFilter::try_from(json!({"fromBlock": "earliest", "toBlock": "0x01"})).unwrap();

        let expected_filter = LogFilter {
            block_filter: Some(BlockFilter::Bounded {
                from_block: Some(BlockNumber::Earliest),
                to_block: Some(BlockNumber::Number(U64::one())),
            }),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);

        let filter = LogFilter::try_from(json!({ "blockHash": get_block_hash_1_str() })).unwrap();
        let expected_filter = LogFilter {
            block_filter: Some(BlockFilter::Exact {
                block_hash: get_block_hash_1(),
            }),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);
    }

    #[test]
    fn test_log_filter_deserialization_address() {
        let filter = LogFilter::try_from(json!({
            "address": [],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            address: Some(LogAddressFilter(vec![])),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);

        let filter = LogFilter::try_from(json!({
            "address": [get_address_str()],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            address: Some(LogAddressFilter(vec![get_address_1()])),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);

        let filter = LogFilter::try_from(json!({
            "address": [get_address_str(), get_address_str()],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            address: Some(LogAddressFilter(vec![get_address_1(), get_address_1()])),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);
    }

    #[test]
    fn test_log_filter_deserialization_topics() {
        let filter = LogFilter::try_from(json!({
            "topics": [],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            topics: Some(vec![]),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);

        let filter = LogFilter::try_from(json!({
            "topics": [null],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            topics: Some(vec![None]),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);

        let filter = LogFilter::try_from(json!({
            "topics": [[get_topic_1_str()]],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            topics: Some(vec![Some(LogTopicFilter(vec![get_topic_1()]))]),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);

        let filter = LogFilter::try_from(json!({
            "topics": [[get_topic_1_str()], null],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            topics: Some(vec![Some(LogTopicFilter(vec![get_topic_1()])), None]),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);

        let filter = LogFilter::try_from(json!({
            "topics": [[get_topic_1_str()], null, [get_topic_1_str(), get_topic_2_str()]],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            topics: Some(vec![
                Some(LogTopicFilter(vec![get_topic_1()])),
                None,
                Some(LogTopicFilter(vec![get_topic_1(), get_topic_2()])),
            ]),
            ..Default::default()
        };
        assert_eq!(filter, expected_filter);
    }

    #[test]
    fn test_log_filter_deserialization_combine() {
        let filter = LogFilter::try_from(json!({
            "blockHash": get_block_hash_1_str(),
            "address": [get_address_str()],
            "topics": [null, [get_topic_1_str()], [get_topic_1_str(), get_topic_2_str()]],
        }))
        .unwrap();
        let expected_filter = LogFilter {
            block_filter: Some(BlockFilter::Exact {
                block_hash: get_block_hash_1(),
            }),
            address: Some(LogAddressFilter(vec![get_address_1()])),
            topics: Some(vec![
                None,
                Some(LogTopicFilter(vec![get_topic_1()])),
                Some(LogTopicFilter(vec![get_topic_1(), get_topic_2()])),
            ]),
        };
        assert_eq!(filter, expected_filter);
    }
}
