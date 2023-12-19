use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use candid::{CandidType, Deserialize};
use ic_canister_client::CanisterClient;
use ic_exports::ic_cdk::api::call;
use jsonrpc_core::{Call, Request, Response};
use reqwest::Url;
use serde::Serialize;
use serde_bytes::ByteBuf;

use crate::outcall::{http_request_required_cycles, HttpOutcall, HttpOutcallArgs};
use crate::{Client, ETH_SEND_RAW_TRANSACTION_METHOD};

use ic_exports::ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpResponse as MHttpResponse,
    TransformContext,
};

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

            let http_response: HttpResponse = if is_update_call {
                client.update("http_request_update", (args,)).await
            } else {
                client.query("http_request", (args,)).await
            }
            .context("failed to send RPC request")?;

            let response = serde_json::from_slice(&http_response.body)
                .context("failed to deserialize RPC request")?;

            log::trace!("response: {:?}", response);

            Ok(response)
        })
    }
}

impl<T: CanisterClient + Sync + 'static> HttpOutcall for T {
    fn http_outcall(
        &self,
        arg: HttpOutcallArgs,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<MHttpResponse>> + Send>> {
        Box::pin(async move {
            let HttpOutcallArgs {
                url,
                method,
                body,
                max_response_bytes,
            } = arg;

            log::trace!("CanisterClient - sending 'http_outcall'. url: {url}");

            let real_url =
                Url::parse(&url).map_err(|e| anyhow::format_err!("error parsing the url {e}"))?;

            let host = real_url
                .host_str()
                .ok_or_else(|| anyhow::format_err!("empty host of url".to_string()))?;

            let headers = vec![
                HttpHeader {
                    name: "Host".to_string(),
                    value: host.to_string(),
                },
                HttpHeader {
                    name: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                },
            ];

            let request = CanisterHttpRequestArgument {
                url,
                max_response_bytes,
                method,
                headers,
                body,
                transform: Some(TransformContext::from_name("transform".to_string(), vec![])),
            };

            let cost = http_request_required_cycles(&request);

            let cycles_available = call::msg_cycles_available128();
            if cycles_available < cost {
                anyhow::bail!("Too few cycles, expected: {cost}, received: {cycles_available}");
            }

            let res = http_request(request, cost)
                .await
                .map(|(res,)| res)
                .map_err(|(r, m)| {
                    anyhow::format_err!(format!("RejectionCode: {r:?}, Error: {m}"))
                })?;

            log::trace!("CanisterClient - Response from http_outcall'. Response : {res:?}");

            Ok(res)
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
