use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use env_logger::Builder;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRpcClient;
use evm_block_extractor::config::ExtractorArgs;
use evm_block_extractor::server::{server_start, server_stop};
use evm_block_extractor::task::block_extractor::start_extractor;
use lightspeed_scheduler::job::Job;
use lightspeed_scheduler::scheduler::Scheduler;
use lightspeed_scheduler::JobExecutor;
use log::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ExtractorArgs::parse();

    // Initialize logger
    init_logger(&config.log_filter)?;

    info!("Emvc Block Extractor");
    info!("----------------------");
    info!("- server_address: {}", config.server_address);
    info!("- remote_rpc_url: {:?}", config.remote_rpc_url);
    info!("- rpc_batch_size: {}", config.rpc_batch_size);
    info!("- request_time_out_secs: {}", config.request_time_out_secs);
    info!(
        "- reset_db_on_state_change: {}",
        config.reset_db_on_state_change
    );
    info!("----------------------");

    let db_client = config.command.clone().build_client().await?;

    let job_executor = JobExecutor::new_with_local_tz();

    // Configure and start the block extractor task
    if let Some(rpc_url) = config.remote_rpc_url.clone() {
        let evm_client = Arc::new(EthJsonRpcClient::new(ReqwestClient::new(rpc_url)));
        let config = config.clone();
        let evm_client = evm_client.clone();
        let db_client = db_client.clone();

        job_executor
            .add_job_with_scheduler(
                Scheduler::Interval {
                    interval_duration: Duration::from_secs(
                        config.block_extractor_job_interval_seconds,
                    ),
                    execute_at_startup: true,
                },
                Job::new("evm_block_extractor", "extract_blocks", None, move || {
                    let config = config.clone();
                    let evm_client = evm_client.clone();
                    let db_client = db_client.clone();
                    Box::pin(async move {
                        start_extractor(config, db_client, evm_client).await?;
                        Ok(())
                    })
                }),
            )
            .await;
    } else {
        warn!("remote_rpc_url is empty, fetching blocks is disabled");
    }

    // Start the job executor
    let _job_executor_handle = job_executor.run().await?;

    // Create EVM client if remote RPC URL is provided
    let evm_client = config
        .remote_rpc_url
        .as_ref()
        .map(|url| Arc::new(EthJsonRpcClient::new(ReqwestClient::new(url.clone()))));

    // Start JSON RPC server
    let server_handle = server_start(&config.server_address, db_client, evm_client).await?;

    // Subscribe to the termination signals
    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            info!("Received shutdown signal");
        }
        Err(err) => error!("Failed to listen for shutdown signal: {err}"),
    }

    // Stop the world
    {
        let stop_gracefully = true;
        job_executor
            .stop(stop_gracefully)
            .await
            .expect("The job executor should stop!");

        server_stop(server_handle).await?;
    }

    Ok(())
}

/// Initialize the logger
fn init_logger(logger_filter: &str) -> Result<(), SetLoggerError> {
    Builder::new().parse_filters(logger_filter).try_init()
}
