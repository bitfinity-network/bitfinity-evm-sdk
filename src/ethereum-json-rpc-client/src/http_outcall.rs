use std::future::Future;
use std::pin::Pin;

use crate::{Client, JsonRpcError, JsonRpcResult};
use did::rpc::request::RpcRequest;
use did::rpc::response::RpcResponse;
use ic_exports::ic_cdk::management_canister::{
    self, HttpHeader, HttpMethod, HttpRequestArgs, TransformContext,
};

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

    /// Sets transform context for the client.
    ///
    /// Transform context is used to sanitize HTTP responses before checking for consensus.
    ///
    /// # Arguments
    /// * `transform_context` - method to use to sanitize HTTP response
    pub fn with_transform(mut self, transform_context: TransformContext) -> Self {
        self.transform_context = Some(transform_context);
        self
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
        request: RpcRequest,
    ) -> Pin<Box<dyn Future<Output = JsonRpcResult<RpcResponse>> + Send>> {
        let url = self.url.clone();
        let max_response_bytes = self.max_response_bytes;
        let body = serde_json::to_vec(&request).expect("failed to serialize body");

        let transform = self.transform_context.clone();
        Box::pin(async move {
            log::trace!("CanisterClient - sending 'http_outcall'. url: {url}");

            let parsed_url = url::Url::parse(&url)?;

            let host = parsed_url
                .host_str()
                .ok_or_else(|| JsonRpcError::UrlMissingHost(parsed_url.clone()))?;

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

            let request = HttpRequestArgs {
                url,
                max_response_bytes,
                method: HttpMethod::POST,
                headers,
                body: Some(body),
                transform,
            };

            let cost = management_canister::cost_http_request(&request);

            let cycles_available = ic_exports::ic_cdk::api::canister_cycle_balance();
            if cycles_available < cost {
                return Err(JsonRpcError::InsufficientCycles {
                    available: cycles_available,
                    cost,
                });
            }

            let http_response = management_canister::http_request(&request).await?;

            log::trace!(
                "CanisterClient - Response from http_outcall'. Response: {} {:?}. Body: {}",
                http_response.status,
                http_response.headers,
                String::from_utf8_lossy(&http_response.body)
            );

            let response = serde_json::from_slice(&http_response.body)?;

            log::trace!("CanisterClient - Deserialized response: {response:?}");

            Ok(response)
        })
    }
}
