#[cfg(feature = "ic-agent-client")]
pub mod agent;

pub mod client;
pub mod error;
pub mod ic_client;

#[cfg(feature = "ic-agent-client")]
pub use agent::{AgentError, IcAgentClient};
pub use client::{CanisterClient, EvmCanisterClient};
pub use error::{CanisterClientError, CanisterClientResult, EvmResult, IcError, IcResult};
pub use ic_client::IcCanisterClient;
