use std::future::Future;
use std::pin::Pin;

use ic_exports::ic_cdk::api::management_canister::http_request::{
    CanisterHttpRequestArgument, HttpMethod, HttpResponse as MHttpResponse,
};

/// http outcall argument
#[derive(Debug)]
pub struct HttpOutcallArgs {
    pub url: String,
    pub method: HttpMethod,
    pub body: Option<Vec<u8>>,
    /// Max response from the http outcall
    ///
    /// # NOTE
    /// As much as this is optional, it is important to set the value
    /// otherwise it will be set to default 2MiB which uses a lot of cycles
    pub max_response_bytes: Option<u64>,
}

/// Trait implementation for Http outcalls for canisters
pub trait HttpOutcall: Clone + Send + Sync {
    fn http_outcall(
        &self,
        args: HttpOutcallArgs,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<MHttpResponse>> + Send>>;
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
