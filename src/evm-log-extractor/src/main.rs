use std::error::Error;
use std::time::Duration;

use clap::Parser;
use env_logger::Builder;
use evm_canister_client::{EvmCanisterClient, IcAgentClient};
use evm_log_extractor::config::LogExtractorConfig;
use evm_log_extractor::job::logs::{run_logs_job, LogsJobSettings};
use lightspeed_scheduler::job::Job;
use lightspeed_scheduler::scheduler::Scheduler;
use lightspeed_scheduler::JobExecutor;
use log::SetLoggerError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = LogExtractorConfig::parse();
    init_logger(&config.logger_filter)?;

    let job_executor = JobExecutor::new_with_local_tz();

    let evmc_client = build_evmc_client(&config)
        .await
        .expect("failed to build evmc client");

    configure_logs_job(evmc_client, &config, &job_executor).await;

    // Start the job executor
    let _job_executor_handle = job_executor.run().await?;

    // Wait for a signal to stop the job executor
    tokio::signal::ctrl_c().await.unwrap();

    // Stop the job executor
    let stop_gracefully = true;
    job_executor
        .stop(stop_gracefully)
        .await
        .expect("The job executor should stop!");

    Ok(())
}

/// Initializes the logger
fn init_logger(logger_filter: &str) -> Result<(), SetLoggerError> {
    // Initialize logger
    Builder::new().parse_filters(logger_filter).try_init()
}

/// Builds the EVMC client if the config is provided
async fn build_evmc_client(
    config: &LogExtractorConfig,
) -> anyhow::Result<EvmCanisterClient<IcAgentClient>> {
    let agent = IcAgentClient::with_identity(
        config.evmc_principal,
        config.identity.clone(),
        &config.evmc_network_url,
        None,
    )
    .await?;
    Ok(EvmCanisterClient::new(agent))
}

/// Configures and starts the logs synchronization job
async fn configure_logs_job(
    evmc_client: EvmCanisterClient<IcAgentClient>,
    config: &LogExtractorConfig,
    job_executor: &JobExecutor,
) {
    // Configure and start the logs job
    let settings = LogsJobSettings {
        path: config.logs_directory.clone(),
        max_logs_per_call: config.logs_synchronization_job_max_logs_per_call,
        start_from_offset: Default::default(),
    };
    let evmc_client = evmc_client.clone();
    job_executor
        .add_job_with_scheduler(
            Scheduler::Interval {
                interval_duration: Duration::from_secs(
                    config.logs_synchronization_job_interval_seconds,
                ),
                execute_at_startup: true,
            },
            Job::new("evm_log_extractor", "logs", None, move || {
                let evmc_client = evmc_client.clone();
                let settings = settings.clone();
                Box::pin(async move {
                    run_logs_job(evmc_client, settings).await?;
                    Ok(())
                })
            }),
        )
        .await;
}
