use candid::{CandidType, Deserialize};
use ic_canister::{generate_idl, init, query, update, Canister, Idl, PreUpdate};
use ic_exports::ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_exports::ic_kit::ic;
use ic_exports::Principal;

use crate::error::{Error, Result};
use crate::state::http::{http, HttpRequest as ServeRequest, HttpResponse as ServeHttpResponse};
use crate::state::{PairKey, Settings, State};
use crate::timer::{sync_price, transform};

/// A canister to transfer funds between IC token canisters and EVM canister contracts.
#[derive(Canister)]
pub struct OracleCanister {
    #[id]
    id: Principal,
    state: State,
}

impl PreUpdate for OracleCanister {}

impl OracleCanister {
    /// Initialize the canister with given data.
    #[init]
    pub fn init(&mut self, init_data: InitData) {
        let settings = Settings {
            owner: init_data.owner,
            evmc_principal: init_data.evmc_principal,
        };

        self.state.reset(settings);

        #[cfg(target_arch = "wasm32")]
        crate::timer::wasm32::init_timer(self.state.pair_price);
    }

    /// Returns principal of canister owner.
    #[query]
    pub fn get_owner(&self) -> Principal {
        self.state.config.get_owner()
    }

    /// Sets a new principal for canister owner.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorised` will be returned.
    #[update]
    pub fn set_owner(&mut self, owner: Principal) -> Result<()> {
        self.check_owner(ic::caller())?;
        self.state.config.set_owner(owner);
        Ok(())
    }

    /// Returns principal of EVM canister with which the minter canister works.
    #[query]
    pub fn get_evmc_principal(&self) -> Principal {
        self.state.config.get_evmc_principal()
    }

    /// Sets principal of EVM canister with which the minter canister works.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorised` will be returned.
    #[update]
    pub fn set_evmc_principal(&mut self, evmc: Principal) -> Result<()> {
        self.check_owner(ic::caller())?;
        self.state.config.set_evmc_principal(evmc);
        Ok(())
    }

    /// Returns the all types of price pairs
    #[query]
    pub fn get_pairs(&self) -> Vec<String> {
        self.state
            .pair_price
            .get_pairs()
            .iter()
            .map(|p| p.0.clone())
            .collect()
    }

    /// Returns the latest (timestamp, price) of given pair
    #[query]
    pub fn get_latest_price(&self, pair: String) -> Result<(u64, u64)> {
        let pair_key = PairKey(pair);
        if !self.state.pair_price.is_exist(&pair_key) {
            return Err(Error::PairNotExist);
        }
        self.state
            .pair_price
            .get_latest_price(&pair_key)
            .ok_or(Error::Internal(
                "latest price for this pair doesn't exist.".to_string(),
            ))
    }

    /// Return the latest n records of a price pair, or fewer if the price's amount fewer
    pub fn get_prices(&self, pair: String, n: usize) -> Vec<(u64, u64)> {
        self.state.pair_price.get_prices(&PairKey(pair), n)
    }

    /// Adds a new pair to the oracle canister.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorised` will be returned.
    ///
    /// If `pair` is used already, `Error::PairExist` will be returned.
    #[update]
    pub fn add_pair(&mut self, pair: String) -> Result<()> {
        self.check_owner(ic::caller())?;
        self.state.pair_price.add_pair(PairKey(pair))
    }

    /// Remove the given pair from the oracle canister.
    ///
    /// This method should be called only by current owner,
    /// else `Error::NotAuthorised` will be returned.
    ///
    /// If there is no pair for `pair`, `Error::PairNotFound` will be returned.
    #[update]
    pub fn remove_pair(&mut self, pair: String) -> Result<()> {
        self.check_owner(ic::caller())?;
        self.state.pair_price.del_pair(PairKey(pair))
    }

    #[update]
    pub async fn update_price(&mut self, pair: String) -> Result<()> {
        self.check_owner(ic::caller())?;
        let now = ic::time();

        let pair_key = PairKey(pair);
        if !self.state.pair_price.is_exist(&pair_key) {
            return Err(Error::PairNotExist);
        }

        sync_price(pair_key, now, &mut self.state.pair_price).await
    }

    #[query]
    fn http_request(&self, req: ServeRequest) -> ServeHttpResponse {
        let now = ic::time();
        http(req, now, &self.state.pair_price)
    }

    fn check_owner(&self, principal: Principal) -> Result<()> {
        let owner = self.state.config.get_owner();
        if owner == principal || owner == Principal::anonymous() {
            return Ok(());
        }
        Err(Error::NotAuthorized)
    }

    /// Requirements for Http outcalls, used to ignore small differences in the data obtained
    /// by different nodes of the IC subnet to reach a consensus, more info:
    /// https://internetcomputer.org/docs/current/developer-docs/integrations/http_requests/http_requests-how-it-works#transformation-function
    #[query]
    fn transform(&self, raw: TransformArgs) -> HttpResponse {
        transform(raw)
    }

    /// Returns candid IDL.
    /// This should be the last fn to see previous endpoints in macro.
    pub fn idl() -> Idl {
        generate_idl!()
    }
}

/// Minter canister initialization data.
#[derive(Debug, Deserialize, CandidType, Clone, Copy)]
pub struct InitData {
    /// Principal of canister's owner.
    pub owner: Principal,

    /// Principal of EVM canister, in which minter canister will mint/burn tokens.
    pub evmc_principal: Principal,
}

// #[cfg(test)]
// mod test {
//     use candid::Principal;
//     use did::H160;
//     use ic_canister::{canister_call, Canister};
//     use ic_exports::ic_kit::{inject, MockContext};

//     use super::InitData;
//     use crate::error::Error;
//     use crate::OracleCanister;

//     fn owner() -> Principal {
//         Principal::from_slice(&[1; 20])
//     }

//     fn bob() -> Principal {
//         Principal::from_slice(&[2; 20])
//     }

//     fn token0() -> Principal {
//         Principal::from_slice(&[3; 20])
//     }

//     fn token1() -> Principal {
//         Principal::from_slice(&[4; 20])
//     }

//     fn contract0() -> H160 {
//         H160::from_slice(&[0; 20])
//     }

//     fn contract1() -> H160 {
//         H160::from_slice(&[1; 20])
//     }

//     async fn init_canister() -> OracleCanister {
//         MockContext::new().inject();

//         const MOCK_PRINCIPAL: &str = "mfufu-x6j4c-gomzb-geilq";
//         let mock_canister_id = Principal::from_text(MOCK_PRINCIPAL).expect("valid principal");
//         let mut canister = OracleCanister::from_principal(mock_canister_id);

//         let init_data = InitData {
//             owner: owner(),
//             evmc_principal: Principal::anonymous(),
//         };
//         canister_call!(canister.init(init_data), ()).await.unwrap();
//         canister
//     }

//     #[tokio::test]
//     async fn correct_initialization() {
//         let canister = init_canister().await;

//         let stored_owner = canister_call!(canister.get_owner(), Principal)
//             .await
//             .unwrap();
//         assert_eq!(stored_owner, owner());

//         let stored_evmc = canister_call!(canister.get_evmc_principal(), Principal)
//             .await
//             .unwrap();
//         assert_eq!(stored_evmc, Principal::anonymous());
//     }

//     #[tokio::test]
//     async fn owner_access_control() {
//         let mut canister = init_canister().await;

//         // try to call with not owner id
//         let set_error = canister_call!(canister.set_owner(bob()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap_err();
//         assert_eq!(set_error, Error::NotAuthorized);

//         // now we will try to call it with owner id
//         inject::get_context().update_id(owner());
//         canister_call!(canister.set_owner(bob()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap();

//         // check if state updated
//         let stored_owner = canister_call!(canister.get_owner(), Principal)
//             .await
//             .unwrap();
//         assert_eq!(stored_owner, bob());
//     }

//     #[tokio::test]
//     async fn evmc_principal_access_control() {
//         let mut canister = init_canister().await;

//         // try to call with not owner id
//         let set_error = canister_call!(canister.set_evmc_principal(bob()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap_err();
//         assert_eq!(set_error, Error::NotAuthorized);

//         // now we will try to call it with owner id
//         inject::get_context().update_id(owner());
//         canister_call!(canister.set_evmc_principal(bob()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap();

//         // check if state updated
//         let stored_owner = canister_call!(canister.get_evmc_principal(), Principal)
//             .await
//             .unwrap();
//         assert_eq!(stored_owner, bob());
//     }

//     #[tokio::test]
//     async fn token_pairs_update_access_control() {
//         let mut canister = init_canister().await;

//         // try to call with not owner id
//         let add_error = canister_call!(canister.add_token_pair(token0(), contract0()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap_err();
//         assert_eq!(add_error, Error::NotAuthorized);
//         let remove_error = canister_call!(canister.remove_token_pair(token0()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap_err();
//         assert_eq!(remove_error, Error::NotAuthorized);

//         // now we will try to call it with owner id
//         inject::get_context().update_id(owner());
//         canister_call!(canister.add_token_pair(token0(), contract0()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap();
//         canister_call!(canister.remove_token_pair(token0()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap();
//     }

//     #[tokio::test]
//     async fn token_pairs_correctly_stored() {
//         let mut canister = init_canister().await;
//         inject::get_context().update_id(owner());

//         canister_call!(canister.add_token_pair(token0(), contract0()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap();
//         canister_call!(canister.add_token_pair(token1(), contract1()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap();

//         // check all addresses stored correctly and available
//         let stored_contract_0 = canister_call!(
//             canister.get_contract_address_by_ic_token(token0()),
//             Option<H160>
//         )
//         .await
//         .unwrap()
//         .unwrap();
//         assert_eq!(stored_contract_0, contract0());

//         let stored_contract_1 = canister_call!(
//             canister.get_contract_address_by_ic_token(token1()),
//             Option<H160>
//         )
//         .await
//         .unwrap()
//         .unwrap();
//         assert_eq!(stored_contract_1, contract1());

//         let stored_token_0 = canister_call!(
//             canister.get_ic_token_by_contract_address(contract0()),
//             Option<Principal>
//         )
//         .await
//         .unwrap()
//         .unwrap();
//         assert_eq!(stored_token_0, token0());

//         let stored_token_1 = canister_call!(
//             canister.get_ic_token_by_contract_address(contract1()),
//             Option<Principal>
//         )
//         .await
//         .unwrap()
//         .unwrap();
//         assert_eq!(stored_token_1, token1());

//         // remove token pair and check
//         canister_call!(canister.remove_token_pair(token0()), Result<()>)
//             .await
//             .unwrap()
//             .unwrap();

//         let stored_contract_0 = canister_call!(
//             canister.get_contract_address_by_ic_token(token0()),
//             Option<H160>
//         )
//         .await
//         .unwrap();
//         assert!(stored_contract_0.is_none());

//         let stored_token_0 = canister_call!(
//             canister.get_ic_token_by_contract_address(contract0()),
//             Option<Principal>
//         )
//         .await
//         .unwrap();
//         assert!(stored_token_0.is_none());
//     }
// }
