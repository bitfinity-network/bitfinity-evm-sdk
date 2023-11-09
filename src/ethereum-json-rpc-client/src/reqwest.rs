
use std::{pin::Pin, future::Future};

use anyhow::Context;
use jsonrpc_core::{Request, Response};
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

// #[async_trait::async_trait]
impl Client for ReqwestClient {

    fn send_rpc_query_request(&self, request: Request) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send + Sync>> {
        let client = self.client.clone();
        let endpoint_url = self.endpoint_url.clone();
        Box::pin(async move {

            log::trace!("ReqwestClient - sending request {request:?}");
            
            let response = client
            .post(&endpoint_url)
            .json(&request)
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

    // async fn send_rpc_update_request(&self, request: Request) -> anyhow::Result<Response> {
    //     Ok(self.send_rpc_query_request(request).await)
    // }

}