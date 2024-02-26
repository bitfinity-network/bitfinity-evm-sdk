use std::path::PathBuf;

use clap::Parser;
use evm_canister_client::ic_agent::export::Principal;

/// A tool that synchronizes blocks from the remote Ethereum JSON RPC endpoint
/// and re-executes it locally.
#[derive(Debug, Clone, Parser)]
pub struct LogExtractorConfig {
    /// Sets the logger [`EnvFilter`].
    /// Valid values: trace, debug, info, warn, error
    /// Example of a valid filter: "warn,my_crate=info,my_crate::my_mod=debug,[my_span]=trace".
    #[clap(long, default_value = "info")]
    pub logger_filter: String,

    /// URL of the EVMC network.
    #[clap(long, default_value = "http://127.0.0.1:8000")]
    pub evmc_network_url: String,

    /// Path to your identity pem file.
    #[arg(long)]
    pub identity: PathBuf,

    /// evmc canister Principal.
    #[arg(long)]
    pub evmc_principal: Principal,

    /// Logs synchronization job interval schedule.
    #[clap(long, default_value = "10")]
    pub logs_synchronization_job_interval_seconds: u64,

    /// Logs synchronization job max logs to download per call.
    #[clap(long, default_value = "5000")]
    pub logs_synchronization_job_max_logs_per_call: usize,

    /// Path to the directory where to put the EVM downloaded logs.
    #[clap(long, default_value = "./")]
    pub logs_directory: String,
}
