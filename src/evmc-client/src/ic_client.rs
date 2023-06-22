use candid::utils::ArgumentEncoder;
use candid::{encode_args, CandidType, Principal};
use did::codec;
use ic_exports::ic_cdk::api::call;
use serde::Deserialize;

use crate::client::CanisterClient;
use crate::{CanisterClientError, CanisterClientResult};

/// This client is used to interact with the IC canister.
#[derive(Debug)]
pub struct IcCanisterClient {
    /// The canister id of the Evm canister
    canister_id: Principal,
}

impl IcCanisterClient {
    pub fn new(canister: Principal) -> Self {
        Self {
            canister_id: canister,
        }
    }
}

#[async_trait::async_trait]
impl CanisterClient for IcCanisterClient {
    async fn update<T, R>(&self, method: &str, args: T) -> CanisterClientResult<R>
    where
        T: ArgumentEncoder + Send,
        R: for<'de> Deserialize<'de> + CandidType,
    {
        let raw_args = encode_args(args)?;
        call::call_raw(self.canister_id, method, raw_args, 0)
            .await
            .map_err(CanisterClientError::CanisterError)
            .map(|r| codec::decode(&r))
    }

    async fn query<T, R>(&self, method: &str, args: T) -> CanisterClientResult<R>
    where
        T: ArgumentEncoder + Send,
        R: for<'de> Deserialize<'de> + CandidType,
    {
        let raw_args = encode_args(args)?;
        call::call_raw(self.canister_id, method, raw_args, 0)
            .await
            .map_err(CanisterClientError::CanisterError)
            .map(|r| codec::decode(&r))
    }
}
