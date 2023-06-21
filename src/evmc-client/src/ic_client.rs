use candid::utils::ArgumentEncoder;
use candid::{encode_args, CandidType, Principal};
use did::codec;
use ic_exports::ic_cdk::api::call;
use serde::Deserialize;

use crate::client::EvmCanisterClient;
use crate::IcResult;

#[derive(Debug)]
pub struct IcCanisterClient {
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
impl EvmCanisterClient for IcCanisterClient {
    async fn update<T, R>(&self, method: &str, args: T) -> IcResult<R>
    where
        T: ArgumentEncoder + Send,
        R: for<'de> Deserialize<'de> + CandidType,
    {
        let raw_args = encode_args(args).expect("encode args failed");
        call::call_raw(self.canister_id, method, raw_args, 0)
            .await
            .map(|r| codec::decode(&r))
    }

    async fn query<T, R>(&self, method: &str, args: T) -> IcResult<R>
    where
        T: ArgumentEncoder + Send,
        R: for<'de> Deserialize<'de> + CandidType,
    {
        let raw_args = encode_args(args).expect("encode args failed");
        call::call_raw(self.canister_id, method, raw_args, 0)
            .await
            .map(|r| codec::decode(&r))
    }
}
