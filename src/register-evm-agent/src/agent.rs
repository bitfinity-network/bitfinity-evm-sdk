use candid::Principal;
use evm_canister_client::ic_agent::Agent;

use crate::error::{Error, Result};

/// Returns `Principal` from ic agent
pub fn user_principal(agent: &Agent) -> Result<Principal> {
    match agent.get_principal() {
        Ok(principal) => Ok(principal),
        Err(e) => Err(Error::CouldNotGetPrincipal(e)),
    }
}
