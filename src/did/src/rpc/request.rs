use alloy::rpc::json_rpc::Request;
use serde::Serialize;

use super::params::Params;
use crate::constant::{JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME, UPGRADE_HTTP_METHODS};

/// Counts of the methods contained in a RpcRequest
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MethodCallCount {
    /// number of read only methods in the RpcRequest
    pub read_only: usize,
    /// number of committable methods in the RpcRequest
    pub commit: usize,
    /// number of mint native token methods in the RpcRequest
    pub mint_native_token: usize,
}

/// Represents jsonrpc request which can be both a batch or a single request
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RpcRequest {
    Batch(Vec<Request<Params>>),
    Single(Request<Params>),
}

impl RpcRequest {
    /// returns the number of read_only and committable methods in the RpcRequest
    pub fn methods_count(&self) -> MethodCallCount {
        match self {
            RpcRequest::Batch(methods) => {
                methods
                    .iter()
                    .fold(MethodCallCount::default(), |mut count, request| {
                        let method = request.meta.method.to_string();
                        if UPGRADE_HTTP_METHODS.contains(&method.as_str()) {
                            count.commit += 1;
                            if method == JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME {
                                count.mint_native_token += 1;
                            }
                            count
                        } else {
                            count.read_only += 1;
                            count
                        }
                    })
            }
            RpcRequest::Single(request) => {
                let mut count = MethodCallCount::default();
                let method = request.meta.method.to_string();
                if UPGRADE_HTTP_METHODS.contains(&method.as_str()) {
                    count.commit += 1;
                    if method == JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME {
                        count.mint_native_token += 1;
                    }
                    count
                } else {
                    count.read_only += 1;
                    count
                }
            }
        }
    }

    /// returns whether the request contains committable methods
    pub fn has_commit_methods(&self) -> bool {
        match self {
            RpcRequest::Batch(methods) => methods.iter().any(|request| {
                UPGRADE_HTTP_METHODS.contains(&request.meta.method.to_string().as_str())
            }),
            RpcRequest::Single(request) => {
                UPGRADE_HTTP_METHODS.contains(&request.meta.method.to_string().as_str())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy::rpc::json_rpc::{Id, RequestMeta};

    use super::*;
    use crate::constant::{IC_SEND_CONFIRM_BLOCK, JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME};

    #[test]
    fn test_single_request_serialization() {
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new("eth_getBalance".into(), Id::Number(1)),
            params: Params::Array(vec![
                serde_json::Value::String("0x123".to_string()),
                serde_json::Value::String("latest".to_string()),
            ]),
        });

        let json = serde_json::to_string(&request).unwrap();
        let expected =
            r#"{"jsonrpc":"2.0","id":1,"method":"eth_getBalance","params":["0x123","latest"]}"#;

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap(),
            serde_json::from_str::<serde_json::Value>(expected).unwrap()
        );
    }

    #[test]
    fn test_batch_request_serialization() {
        let request = RpcRequest::Batch(vec![
            Request {
                meta: RequestMeta::new("eth_getBalance".into(), Id::Number(1)),
                params: Params::Array(vec![
                    serde_json::Value::String("0x123".to_string()),
                    serde_json::Value::String("latest".to_string()),
                ]),
            },
            Request {
                meta: RequestMeta::new(
                    JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME.into(),
                    Id::String("second-call".to_string()),
                ),
                params: Params::Array(vec![serde_json::Value::String(
                    "0xrawTransaction".to_string(),
                )]),
            },
        ]);

        let json = serde_json::to_string(&request).unwrap();
        // Verify it's a batch by checking it starts with "[" and ends with "]"
        assert!(json.trim().starts_with('['));
        assert!(json.trim().ends_with(']'));

        // Parse back and verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let array = parsed.as_array().unwrap();
        assert_eq!(array.len(), 2);
        assert_eq!(array[0]["method"], "eth_getBalance");
        assert_eq!(
            array[1]["method"],
            JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME
        );
    }

    #[test]
    fn test_methods_count_single_read_only() {
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new("eth_getBalance".into(), Id::Number(1)),
            params: Params::Array(vec![]),
        });

        let count = request.methods_count();
        assert_eq!(count.read_only, 1);
        assert_eq!(count.commit, 0);
        assert_eq!(count.mint_native_token, 0);
    }

    #[test]
    fn test_methods_count_single_commit() {
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new(
                JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME.into(),
                Id::Number(1),
            ),
            params: Params::Array(vec![]),
        });

        let count = request.methods_count();
        assert_eq!(count.read_only, 0);
        assert_eq!(count.commit, 1);
        assert_eq!(count.mint_native_token, 0);
    }

    #[test]
    fn test_methods_count_single_mint_native_token() {
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new(
                JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME.into(),
                Id::Number(1),
            ),
            params: Params::Array(vec![]),
        });

        let count = request.methods_count();
        assert_eq!(count.read_only, 0);
        assert_eq!(count.commit, 1);
        assert_eq!(count.mint_native_token, 1);
    }

    #[test]
    fn test_methods_count_batch_mixed() {
        let request = RpcRequest::Batch(vec![
            // Read only
            Request {
                meta: RequestMeta::new("eth_getBalance".into(), Id::Number(1)),
                params: Params::Array(vec![]),
            },
            // Read only
            Request {
                meta: RequestMeta::new("eth_blockNumber".into(), Id::Number(2)),
                params: Params::Array(vec![]),
            },
            // Commit
            Request {
                meta: RequestMeta::new(
                    JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME.into(),
                    Id::Number(3),
                ),
                params: Params::Array(vec![]),
            },
            // Commit with mint
            Request {
                meta: RequestMeta::new(
                    JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME.into(),
                    Id::Number(4),
                ),
                params: Params::Array(vec![]),
            },
            // Another commit
            Request {
                meta: RequestMeta::new(IC_SEND_CONFIRM_BLOCK.into(), Id::Number(5)),
                params: Params::Array(vec![]),
            },
        ]);

        let count = request.methods_count();
        assert_eq!(count.read_only, 2);
        assert_eq!(count.commit, 3);
        assert_eq!(count.mint_native_token, 1);
    }

    #[test]
    fn test_methods_count_batch_empty() {
        let request = RpcRequest::Batch(vec![]);
        let count = request.methods_count();
        assert_eq!(count.read_only, 0);
        assert_eq!(count.commit, 0);
        assert_eq!(count.mint_native_token, 0);
    }

    #[test]
    fn test_has_commit_methods_single_read_only() {
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new("eth_getBalance".into(), Id::Number(1)),
            params: Params::Array(vec![]),
        });

        assert!(!request.has_commit_methods());
    }

    #[test]
    fn test_has_commit_methods_single_commit() {
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new(
                JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME.into(),
                Id::Number(1),
            ),
            params: Params::Array(vec![]),
        });

        assert!(request.has_commit_methods());
    }

    #[test]
    fn test_has_commit_methods_batch_all_read_only() {
        let request = RpcRequest::Batch(vec![
            Request {
                meta: RequestMeta::new("eth_getBalance".into(), Id::Number(1)),
                params: Params::Array(vec![]),
            },
            Request {
                meta: RequestMeta::new("eth_blockNumber".into(), Id::Number(2)),
                params: Params::Array(vec![]),
            },
        ]);

        assert!(!request.has_commit_methods());
    }

    #[test]
    fn test_has_commit_methods_batch_with_commit() {
        let request = RpcRequest::Batch(vec![
            Request {
                meta: RequestMeta::new("eth_getBalance".into(), Id::Number(1)),
                params: Params::Array(vec![]),
            },
            Request {
                meta: RequestMeta::new(
                    JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME.into(),
                    Id::Number(2),
                ),
                params: Params::Array(vec![]),
            },
        ]);

        assert!(request.has_commit_methods());
    }

    #[test]
    fn test_has_commit_methods_batch_empty() {
        let request = RpcRequest::Batch(vec![]);
        assert!(!request.has_commit_methods());
    }

    #[test]
    fn test_different_params_types() {
        // Object params
        let params = Params::Map(serde_json::Map::from_iter([
            (
                "from".to_string(),
                serde_json::Value::String("0x123".to_string()),
            ),
            (
                "to".to_string(),
                serde_json::Value::String("0x456".to_string()),
            ),
            (
                "value".to_string(),
                serde_json::Value::String("0x1".to_string()),
            ),
        ]));

        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new("eth_call".into(), Id::Number(1)),
            params: params.clone(),
        });

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify params were serialized as object
        assert!(parsed["params"].is_object());
        assert_eq!(parsed["params"]["from"], "0x123");
        assert_eq!(parsed["params"]["to"], "0x456");
        assert_eq!(parsed["params"]["value"], "0x1");

        // Empty array params
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new("eth_blockNumber".into(), Id::Number(2)),
            params: Params::Array(vec![]),
        });

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify params were serialized as empty array
        assert!(parsed["params"].is_array());
        assert_eq!(parsed["params"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_different_id_types() {
        // Number ID
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new("eth_getBalance".into(), Id::Number(123)),
            params: Params::Array(vec![]),
        });

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["id"], 123);

        // String ID
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new(
                "eth_getBalance".into(),
                Id::String("test-request".to_string()),
            ),
            params: Params::Array(vec![]),
        });

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["id"], "test-request");

        // Null ID
        let request = RpcRequest::Single(Request {
            meta: RequestMeta::new("eth_getBalance".into(), Id::None),
            params: Params::Array(vec![]),
        });

        let json = serde_json::to_string(&request).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["id"].is_null());
    }
}
