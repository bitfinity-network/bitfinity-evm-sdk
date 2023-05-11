use anyhow::{bail, Context, Result};
use candid::Principal;
use ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport;
use ic_agent::identity::BasicIdentity;
use ic_agent::Agent;
use std::path::PathBuf;

/// Returns `BasicIdentity` given path
fn get_identity(path: &str) -> Result<BasicIdentity> {
    let path_buf = PathBuf::from(path);
    let identity = BasicIdentity::from_pem_file(&path_buf)
        .with_context(|| format!("Failed to read PEM file: {}", path_buf.display()))?;

    Ok(identity)
}

/// Initialize an IC Agent
pub async fn init_agent(identity_path: &str, network: &str) -> Result<Agent> {
    let identity = get_identity(identity_path)?;

    let url = network_url(network);
    let transport = ReqwestHttpReplicaV2Transport::create(url)?;

    let agent = Agent::builder()
        .with_transport(transport)
        .with_identity(identity)
        .build()?;

    agent.fetch_root_key().await?;

    Ok(agent)
}

/// Returns `Principal` from ic agent
pub fn user_principal(agent: &Agent) -> Result<Principal> {
    match agent.get_principal() {
        Ok(principal) => Ok(principal),
        Err(_) => bail!("failed to get user principal"),
    }
}

pub fn network_url(network: &str) -> &str {
    match network {
        "local" => "http://localhost:8000",
        "ic" => "https://ic0.app",
        url => url,
    }
}
