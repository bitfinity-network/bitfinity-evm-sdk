use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use jsonrpc_core::Response;
pub use reqwest;

use crate::{Client, ClientRequest};

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
    fn send_request(
        &self,
        request: ClientRequest,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send>> {
        log::trace!("ReqwestClient - sending request {request:?}");

        let request = match request {
            ClientRequest::RpcRequest(req) => req,
            ClientRequest::HttpOutCall(_) => unreachable!(),
        };

        let request_builder = self.client.post(&self.endpoint_url).json(&request);

        Box::pin(async move {
            let response = request_builder
                .send()
                .await
                .context("failed to send RPC request")?
                .json::<Response>()
                .await
                .context("failed to decode RPC response")?;

            log::trace!("response: {:?}", response);
            Ok(response)
        })
    }
}
