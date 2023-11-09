use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::Context;
use candid::{CandidType, Deserialize};
use ic_canister_client::StateMachineCanisterClient;
use jsonrpc_core::{Request, Response};
use serde::Serialize;
use serde_bytes::ByteBuf;

use crate::Client;

impl Client for StateMachineCanisterClient {
    fn send_rpc_query_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send + Sync>> {
        let client = self.clone();
        Box::pin(async move { send_request(&client, "http_request", request).await })
    }

    fn send_rpc_update_request(
        &self,
        request: Request,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Response>> + Send + Sync>> {
        let client = self.clone();
        Box::pin(async move { send_request(&client, "http_request_update", request).await })
    }
}

async fn send_request(
    client: &StateMachineCanisterClient,
    method: &'static str,
    request: Request,
) -> anyhow::Result<Response> {
    log::trace!("CanisterClient - sending {method}. request: {request:?}");

    let args = HttpRequest::new(&request)?;

    let http_response: HttpResponse = client
        .query(method, (args,))
        .await
        .context("failed to send RPC request")?;

    let response =
        serde_json::from_slice(&http_response.body).context("failed to deserialize RPC request")?;

    log::trace!("response: {:?}", response);

    Ok(response)
}

/// The important components of an HTTP request.
#[derive(Clone, Debug, CandidType, Deserialize)]
struct HttpRequest {
    /// The HTTP method string.
    pub method: &'static str,
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
