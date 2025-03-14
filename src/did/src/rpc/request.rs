use alloy::rpc::json_rpc::Request;
use serde::{Deserialize, Serialize};

use crate::constant::{JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME, UPGRADE_HTTP_METHODS};

use super::params::Params;

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
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
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
