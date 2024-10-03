use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use ic_exports::ic_cdk::api::call;
use ic_exports::ic_cdk::api::management_canister::http_request::{
    self, CanisterHttpRequestArgument, HttpHeader, HttpMethod, TransformContext,
};
use jsonrpc_core::Request;

use crate::Client;

/// Http outcall client implementation.
#[derive(Debug, Clone)]
pub struct HttpOutcallClient {
    url: String,
    max_response_bytes: Option<u64>,
}

impl HttpOutcallClient {
    /// Creates a new client.
    ///
    /// # Arguments
    /// * `url` - The url of the canister.
    ///
    pub fn new(url: String) -> Self {
        Self {
            url,
            max_response_bytes: None,
        }
    }

    /// The maximal size of the response in bytes. If None, 2MiB will be the
    /// limit.
    /// This value affects the cost of the http request and it is highly
    /// recommended to set it as low as possible to avoid unnecessary extra
    /// costs.
    ///
    /// # Arguments
    /// * `max_response_bytes` - The max response bytes.
    pub fn set_max_response_bytes(&mut self, max_response_bytes: Option<u64>) {
        self.max_response_bytes = max_response_bytes;
    }
}

impl Client for HttpOutcallClient {
    fn send_rpc_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<jsonrpc_core::Response>> + Send>> {
        let url = self.url.clone();
        let max_response_bytes = self.max_response_bytes;
        let body = serde_json::to_vec(&request).expect("failed to serialize body");

        Box::pin(async move {
            log::trace!("CanisterClient - sending 'http_outcall'. url: {url}");

            let parsed_url = url::Url::parse(&url)
                .map_err(|e| anyhow::format_err!("failed to parse url `{url}`: {e}"))?;

            let host = parsed_url
                .host_str()
                .ok_or_else(|| anyhow::format_err!("no host in url `{parsed_url}`"))?;

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
                method: HttpMethod::POST,
                headers,
                body: Some(body),
                transform: Some(TransformContext::from_name("transform".to_string(), vec![])),
            };

            let cost = http_request_required_cycles(&request);

            let cycles_available = ic_exports::ic_cdk::api::canister_balance128();
            if cycles_available < cost {
                anyhow::bail!("Too few cycles, expected: {cost}, available: {cycles_available}");
            }

            let http_response = http_request::http_request(request, cost)
                .await
                .map(|(res,)| res)
                .map_err(|(r, m)| {
                    anyhow::format_err!(format!("RejectionCode: {r:?}, Error: {m}"))
                })?;

            let response = serde_json::from_slice(&http_response.body)
                .context("failed to deserialize RPC request")?;

            log::trace!("CanisterClient - Response from http_outcall'. Response : {response:?}");

            Ok(response)
        })
    }
}

// Calculate cycles for http_request
// NOTE:
// https://github.com/dfinity/cdk-rs/blob/710a6cdcc3eb03d2392df1dfd5f047dff9deee80/examples/management_canister/src/caller/lib.rs#L7-L19
pub fn http_request_required_cycles(arg: &CanisterHttpRequestArgument) -> u128 {
    let max_response_bytes = match arg.max_response_bytes {
        Some(ref n) => *n as u128,
        None => 2 * 1024 * 1024u128, // default 2MiB
    };
    let arg_raw = candid::utils::encode_args((arg,)).expect("Failed to encode arguments.");
    // The fee is for a 13-node subnet to demonstrate a typical usage.
    (3_000_000u128
        + 60_000u128 * 13
        + (arg_raw.len() as u128 + "http_request".len() as u128) * 400
        + max_response_bytes * 800)
        * 13
}
