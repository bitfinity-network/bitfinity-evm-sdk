use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use candid::{CandidType, Deserialize};
use ic_canister_client::CanisterClient;
use jsonrpc_core::{Call, Request, Response};
use serde::Serialize;
use serde_bytes::ByteBuf;

use crate::{Client, UPGRADE_HTTP_METHODS};

impl<T: CanisterClient + Sync + 'static> Client for T {
    fn send_rpc_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send>> {
        let client = self.clone();

        Box::pin(async move {
            log::trace!("CanisterClient - sending 'http_request'. request: {request:?}");

            let is_update_call = match &request {
                Request::Single(Call::MethodCall(call)) => is_update_call(&call.method),
                Request::Batch(calls) => calls.iter().any(|call| {
                    if let Call::MethodCall(call) = call {
                        is_update_call(&call.method)
                    } else {
                        false
                    }
                }),
                _ => false,
            };

            let args = HttpRequest::new(&request)?;

            let http_response: Result<HttpResponse, _> = if is_update_call {
                client.update("http_request_update", (args,)).await
            } else {
                client.query("http_request", (args,)).await
            }
            .map_err(anyhow::Error::from);

            let http_response = match http_response {
                Ok(response) => response,
                Err(e) => {
                    log::warn!("failed to send RPC request: {e}");
                    return Err(e);
                }
            };

            let response = serde_json::from_slice(&http_response.body)
                .context("failed to deserialize RPC request")?;

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
    pub fn new<T: ?Sized + Serialize>(data: &T) -> anyhow::Result<Self> {
        let mut headers = HashMap::new();
        headers.insert("content-type", "application/json");
        Ok(Self {
            method: "POST",
            headers,
            url: "",
            body: ByteBuf::from(
                serde_json::to_vec(data).context("failed to serialize RPC request")?,
            ),
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
