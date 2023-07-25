use std::path::Path;

use candid::Principal;
use evm_canister_client::ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport;
use evm_canister_client::ic_agent::Agent;

mod generic_identity;
use generic_identity::GenericIdentity;

use crate::error::{Error, Result};

/// Initialize an IC Agent
pub async fn init_agent(identity_path: &Path, url: &str) -> Result<Agent> {
    info!("parsing identity from {}", identity_path.display());
    let identity = GenericIdentity::try_from(identity_path)?;
    info!("identity parsed");

    info!("network url: {url}");
    let transport = ReqwestHttpReplicaV2Transport::create(url)?;

    let agent = Agent::builder()
        .with_transport(transport)
        .with_identity(identity)
        .build()?;

    info!("agent built; fetching root key...");
    agent.fetch_root_key().await?;
    info!("agent initialized");

    Ok(agent)
}

/// Returns `Principal` from ic agent
pub fn user_principal(agent: &Agent) -> Result<Principal> {
    match agent.get_principal() {
        Ok(principal) => Ok(principal),
        Err(e) => Err(Error::CouldNotGetPrincipal(e)),
    }
}
