use std::time::Duration;

use did::rpc::request::RpcRequest;
use did::rpc::response::{Response, RpcResponse};
use ethereum_json_rpc_client::Client;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use rand::SeedableRng as _;

/// Public ethereum endpoints which can be used to send RPC requests.
const PUBLIC_ETHEREUM_JSON_API_ENDPOINTS: &[&str] = &[
    "https://cloudflare-eth.com/",
    "https://ethereum.publicnode.com",
    "https://rpc.ankr.com/eth",
    "https://nodes.mewapi.io/rpc/eth",
    "https://eth-mainnet.gateway.pokt.network/v1/5f3453978e354ab992c4da79",
    "https://eth-mainnet.nodereal.io/v1/1659dfb40aa24bbb8153a677b98064d7",
    "https://eth.llamarpc.com",
    "https://eth-mainnet.public.blastapi.io",
];

#[derive(Clone)]
pub enum RpcReqwestClient {
    Public(PublicRpcReqwestClient),
    Alchemy(AlchemyRpcReqwestClient),
}

impl RpcReqwestClient {
    pub fn alchemy(apikey: String) -> Self {
        RpcReqwestClient::Alchemy(AlchemyRpcReqwestClient { apikey })
    }

    pub fn public() -> Self {
        RpcReqwestClient::Public(PublicRpcReqwestClient)
    }
}

impl Client for RpcReqwestClient {
    fn send_rpc_request(
        &self,
        request: RpcRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<RpcResponse>> + Send>>
    {
        match self {
            RpcReqwestClient::Public(client) => client.send_rpc_request(request),
            RpcReqwestClient::Alchemy(client) => client.send_rpc_request(request),
        }
    }
}

/// This client randomly shuffle RPC providers and tries to send the request to each one of them
/// until it gets a successful response.
/// This was necessary because some RPC providers have rate limits and running the CI was more like a nightmare.
#[derive(Clone)]
pub struct PublicRpcReqwestClient;

impl Client for PublicRpcReqwestClient {
    fn send_rpc_request(
        &self,
        request: RpcRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<RpcResponse>> + Send>>
    {
        Box::pin(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();

            use rand::seq::SliceRandom;
            let mut err = None;
            let mut endpoints = PUBLIC_ETHEREUM_JSON_API_ENDPOINTS.to_vec();
            endpoints.shuffle(&mut rng);
            for rpc_endpoint in endpoints {
                let client = ReqwestClient::new_with_client(
                    rpc_endpoint.to_string(),
                    reqwest::ClientBuilder::new()
                        .timeout(Duration::from_secs(10))
                        .build()
                        .unwrap(),
                );
                let result = client.send_rpc_request(request.clone()).await;

                match result {
                    Ok(RpcResponse::Single(Response::Success(_))) => return result,
                    Ok(RpcResponse::Batch(batch))
                        if batch
                            .iter()
                            .all(|output| matches!(output, Response::Success(_))) =>
                    {
                        return Ok(RpcResponse::Batch(batch));
                    }
                    Ok(result) => {
                        err = Some(anyhow::anyhow!("call failed: {result:?}"));
                    }
                    Err(e) => {
                        err = Some(e);
                    }
                }
            }

            Err(err.unwrap())
        })
    }
}

/// Rpc reqwest client which uses the Alchemy API which is very reliable and has a high rate limit.
/// Always use this in CI!!!
#[derive(Clone)]
pub struct AlchemyRpcReqwestClient {
    apikey: String,
}

impl AlchemyRpcReqwestClient {
    /// Get endpoint for Alchemy API
    #[inline]
    fn endpoint(&self) -> String {
        format!("https://eth-mainnet.alchemyapi.io/v2/{}", self.apikey)
    }
}

impl Client for AlchemyRpcReqwestClient {
    fn send_rpc_request(
        &self,
        request: RpcRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<RpcResponse>> + Send>>
    {
        let endpoint = self.endpoint();

        Box::pin(async move {
            let client = ReqwestClient::new_with_client(
                endpoint,
                reqwest::ClientBuilder::new()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .unwrap(),
            );
            let result = client.send_rpc_request(request.clone()).await;

            match result {
                Ok(RpcResponse::Single(Response::Success(_))) => result,
                Ok(RpcResponse::Batch(batch))
                    if batch
                        .iter()
                        .all(|output| matches!(output, Response::Success(_))) =>
                {
                    Ok(RpcResponse::Batch(batch))
                }
                Ok(result) => {
                    anyhow::bail!("call failed: {result:?}")
                }
                Err(e) => {
                    anyhow::bail!("call failed: {e}")
                }
            }
        })
    }
}
