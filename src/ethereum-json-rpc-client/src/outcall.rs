use std::future::Future;
use std::pin::Pin;

use ic_exports::ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod,
    HttpResponse as MHttpResponse, TransformArgs, TransformContext,
};

pub const INGRESS_OVERHEAD_BYTES: u128 = 100;
pub const INGRESS_MESSAGE_RECEIVED_COST: u128 = 1_200_000;
pub const INGRESS_MESSAGE_BYTE_RECEIVED_COST: u128 = 2_000;
pub const HTTP_OUTCALL_REQUEST_COST: u128 = 400_000_000;
pub const HTTP_OUTCALL_BYTE_RECEIVED_COST: u128 = 100_000;
pub const DEFAULT_NODES_IN_SUBNET: u32 = 13;

pub fn transform(raw: TransformArgs) -> MHttpResponse {
    MHttpResponse {
        status: raw.response.status,
        body: raw.response.body,
        ..Default::default()
    }
}

/// http outcall argument
pub struct HttpOutcallArgs {
    pub url: String,
    pub method: HttpMethod,
    pub body: Option<Vec<u8>>,
    /// Cost of the http outcall
    pub cost: Option<u128>,
    /// Max response from the call
    pub max_response_bytes: Option<u64>,
}

impl HttpOutcallArgs {
    pub fn get_request_costs(&self) -> u128 {
        let ingress_bytes = (self.body.clone().unwrap_or_default().len() + self.url.len()) as u128
            + INGRESS_OVERHEAD_BYTES;

        INGRESS_MESSAGE_RECEIVED_COST
            + INGRESS_MESSAGE_BYTE_RECEIVED_COST * ingress_bytes
            + HTTP_OUTCALL_REQUEST_COST
            + HTTP_OUTCALL_BYTE_RECEIVED_COST
                * (ingress_bytes + self.max_response_bytes.unwrap_or(8_000) as u128)
    }
}

/// Trait implementation for Http outcalls for canisters
pub trait HttpOutcall: Clone + Send + Sync {
    fn http_outcall(
        &self,
        args: HttpOutcallArgs,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<MHttpResponse>> + Send>>;
}
