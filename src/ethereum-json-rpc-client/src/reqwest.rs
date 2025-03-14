use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use did::rpc::request::RpcRequest;
use did::rpc::response::RpcResponse;
pub use reqwest;

use crate::Client;

/// Reqwest client implementation.
#[derive(Clone)]
pub struct ReqwestClient {
    client: reqwest::Client,
    endpoint_url: String,
}

impl ReqwestClient {
    /// Creates a new client.
    pub fn new(endpoint_url: String) -> Self {
        Self::new_with_client(endpoint_url, Default::default())
    }

    /// Creates a new client with a custom reqwest client.
    pub fn new_with_client(endpoint_url: String, client: reqwest::Client) -> Self {
        Self {
            endpoint_url,
            client,
        }
    }
}

impl Client for ReqwestClient {
    fn send_rpc_request(
        &self,
        request: RpcRequest,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<RpcResponse>> + Send>> {
        log::trace!("ReqwestClient - sending request {request:?}");

        let request_builder = self.client.post(&self.endpoint_url).json(&request);

        Box::pin(async move {
            let response = request_builder
                .send()
                .await
                .context("failed to send RPC request")?;

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                anyhow::bail!("RPC request failed: {status} - {text}");
            }

            let json_response = response
                .json::<RpcResponse>()
                .await
                .context("failed to decode RPC response")?;

            log::trace!("response: {:?}", json_response);
            Ok(json_response)
        })
    }
}
