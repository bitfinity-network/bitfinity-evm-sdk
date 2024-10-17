use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use ic_cdk::api::management_canister::http_request::{
    self, CanisterHttpRequestArgument, HttpHeader, HttpMethod, TransformContext,
};
#[cfg(feature = "sanitize-http-outcall")]
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_exports::ic_cdk;
use jsonrpc_core::Request;

use crate::Client;

/// EVM client that uses HTTPS Outcalls to communicate with EVM.
///
/// This client can be used to connect to external EVM RPC services, but due to IC HTTPS Outcall
/// protocol interface, there are a few limitations that these services must comply with:
/// * Identical requests must produce identical response bodies.
/// * If some of the response headers may vary for different requests, response transformation must
///   be used (see [HttpOutcallClient::new_with_transform]).
/// * The service must not use redirects or any other form of indirection. All valid RPC requests
///   must be answered with `200 OK` status code and a valid RPC response.
/// * The service must support batched RPC requests.
///
/// Note that when a canister is run in replicated mode (e.g. on IC mainneet), every call to
/// [`HttpClient::send_rpc_request`] will result in multiple concurrent identical HTTP POST requests
/// to the target EVM.
///
/// For more information about how IC HTTPS Outcalls work see [IC documentation](https://internetcomputer.org/docs/current/tutorials/developer-journey/level-3/3.2-https-outcalls/)
#[derive(Debug, Clone)]
pub struct HttpOutcallClient {
    url: String,
    max_response_bytes: Option<u64>,
    #[allow(dead_code)]
    transform_context: Option<TransformContext>,
}

impl HttpOutcallClient {
    /// Creates a new client.
    ///
    /// # Arguments
    /// * `url` - the url of the RPC service to connect to.
    pub fn new(url: String) -> Self {
        Self {
            url,
            max_response_bytes: None,
            transform_context: None,
        }
    }

    /// Creates a new client.
    ///
    /// You can use [`new_sanitized`] method to use default transform context. (Available with
    /// Cargo feature `sanitize-http-outcall`)
    ///
    /// # Arguments
    /// * `url` - the url of the RPC service to connect to.
    /// * `transform_context` - method to use to sanitize HTTP response
    pub fn new_with_transform(url: String, transform_context: TransformContext) -> Self {
        Self {
            url,
            max_response_bytes: None,
            transform_context: Some(transform_context),
        }
    }

    /// Creates a new client with default sanitize method.
    ///
    /// The default sanitize drops most of HTTP headers that may prevent consensus on the response.
    ///
    /// Only available with Cargo feature `sanitize-http-outcall`.
    ///
    /// # Arguments
    /// * `url` - the url of the RPC service to connect to.
    #[cfg(feature = "sanitize-http-outcall")]
    pub fn new_sanitized(url: String) -> Self {
        Self {
            url,
            max_response_bytes: None,
            transform_context: Some(TransformContext::from_name(
                "sanitize_http_response".into(),
                vec![],
            )),
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

#[cfg(feature = "sanitize-http-outcall")]
#[ic_cdk::query]
fn sanitize_http_response(raw_response: TransformArgs) -> HttpResponse {
    const USE_HEADERS: &[&str] = &["content-encoding", "content-length", "content-type", "host"];
    let TransformArgs { mut response, .. } = raw_response;
    response
        .headers
        .retain(|header| USE_HEADERS.iter().any(|v| v == &header.name.to_lowercase()));

    response
}

impl Client for HttpOutcallClient {
    fn send_rpc_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<jsonrpc_core::Response>> + Send>> {
        let url = self.url.clone();
        let max_response_bytes = self.max_response_bytes;
        let body = serde_json::to_vec(&request).expect("failed to serialize body");

        let transform = self.transform_context.clone();
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
            log::trace!("Making http request to {url} with headers: {headers:?}");
            log::trace!("Request body is: {}", String::from_utf8_lossy(&body));

            let request = CanisterHttpRequestArgument {
                url,
                max_response_bytes,
                method: HttpMethod::POST,
                headers,
                body: Some(body),
                transform,
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

            log::trace!(
                "CanisterClient - Response from http_outcall'. Response: {} {:?}. Body: {}",
                http_response.status,
                http_response.headers,
                String::from_utf8_lossy(&http_response.body)
            );

            let response = serde_json::from_slice(&http_response.body)
                .context("failed to deserialize RPC response")?;

            log::trace!("CanisterClient - Deserialized response: {response:?}");

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

#[cfg(test)]
mod tests {
    use candid::Nat;

    use super::*;

    #[cfg(feature = "sanitize-http-outcall")]
    #[test]
    fn sanitize_http_response_removes_extra_headers() {
        let transform_args = TransformArgs {
            response: HttpResponse {
                status: 200u128.into(),
                headers: vec![
                    HttpHeader {
                        name: "content-type".to_string(),
                        value: "application/json".to_string(),
                    },
                    HttpHeader {
                        name: "content-length".to_string(),
                        value: "42".to_string(),
                    },
                    HttpHeader {
                        name: "content-encoding".to_string(),
                        value: "gzip".to_string(),
                    },
                    HttpHeader {
                        name: "date".to_string(),
                        value: "Fri, 11 Oct 2024 10:25:08 GMT".to_string(),
                    },
                ],
                body: vec![],
            },
            context: vec![],
        };

        let sanitized: HttpResponse = sanitize_http_response(transform_args);
        assert_eq!(sanitized.headers.len(), 3);
        assert_eq!(sanitized.status, Nat::from(200u128));
        assert!(sanitized
            .headers
            .iter()
            .any(|header| header.name == "content-type"));
        assert!(!sanitized.headers.iter().any(|header| header.name == "date"));
    }
}
