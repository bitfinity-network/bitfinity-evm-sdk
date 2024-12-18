use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use did::http::{HttpRequest, HttpResponse};
use ic_canister_client::CanisterClient;
use jsonrpc_core::{Call, Request, Response};

use crate::{Client, ETH_SEND_RAW_TRANSACTION_METHOD};

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

            let args = HttpRequest::new(&request);

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

#[inline]
fn is_update_call(method: &str) -> bool {
    method.eq(ETH_SEND_RAW_TRANSACTION_METHOD)
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::ETH_CHAIN_ID_METHOD;

    #[test]
    fn test_is_update_call() {
        assert!(is_update_call(ETH_SEND_RAW_TRANSACTION_METHOD));
        assert!(!is_update_call(ETH_CHAIN_ID_METHOD));
    }
}
