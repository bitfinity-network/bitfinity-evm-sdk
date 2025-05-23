use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use candid::{CandidType, Deserialize};
use did::constant::UPGRADE_HTTP_METHODS;
use did::rpc::request::RpcRequest;
use did::rpc::response::RpcResponse;
use ic_canister_client::CanisterClient;
use serde::Serialize;
use serde_bytes::ByteBuf;

use crate::{Client, JsonRpcError, JsonRpcResult};

impl<T: CanisterClient + Sync + 'static> Client for T {
    fn send_rpc_request(
        &self,
        request: RpcRequest,
    ) -> Pin<Box<dyn Future<Output = JsonRpcResult<RpcResponse>> + Send>> {
        let client = self.clone();

        Box::pin(async move {
            log::trace!("CanisterClient - sending 'http_request'. request: {request:?}");

            let is_update_call = match &request {
                RpcRequest::Single(request) => is_update_call(&request.method),
                RpcRequest::Batch(calls) => calls.iter().any(|call| is_update_call(&call.method)),
            };

            let args = HttpRequest::new(&request)?;

            let http_response: HttpResponse = if is_update_call {
                client.update("http_request_update", (args,)).await
            } else {
                client.query("http_request", (args,)).await
            }
            .map_err(|e| {
                log::warn!("failed to send RPC request: {e}");
                JsonRpcError::CanisterClient(e)
            })?;

            let response = serde_json::from_slice(&http_response.body)?;

            log::trace!("response: {:?}", response);

            Ok(response)
        })
    }
}

/// The important components of an HTTP request.
#[derive(Clone, Debug, CandidType)]
struct HttpRequest {
    /// The HTTP method string.
    pub method: &'static str,
    /// The URL method string.
    pub url: &'static str,
    /// The request headers.
    pub headers: HashMap<&'static str, &'static str>,
    /// The request body.
    pub body: ByteBuf,
}

impl HttpRequest {
    pub fn new<T: ?Sized + Serialize>(data: &T) -> JsonRpcResult<Self> {
        let mut headers = HashMap::new();
        headers.insert("content-type", "application/json");
        Ok(Self {
            method: "POST",
            headers,
            url: "",
            body: ByteBuf::from(serde_json::to_vec(data)?),
        })
    }
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct HttpResponse {
    /// The HTTP status code.
    pub status_code: u16,
    /// The response header map.
    pub headers: HashMap<String, String>,
    /// The response body.
    pub body: ByteBuf,
}

#[inline]
fn is_update_call(method: &str) -> bool {
    UPGRADE_HTTP_METHODS.contains(&method)
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{ETH_CHAIN_ID_METHOD, ETH_SEND_RAW_TRANSACTION_METHOD, IC_SEND_CONFIRM_BLOCK};

    #[test]
    fn test_is_update_call() {
        assert!(is_update_call(ETH_SEND_RAW_TRANSACTION_METHOD));
        assert!(is_update_call(IC_SEND_CONFIRM_BLOCK));
        assert!(!is_update_call(ETH_CHAIN_ID_METHOD));
    }
}
