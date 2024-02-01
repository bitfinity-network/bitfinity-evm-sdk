use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use env_logger::Builder;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use evm_block_extractor::block_extractor::start_extractor;
use evm_block_extractor::config::ExtractorArgs;
use evm_block_extractor::server::{server_start, server_stop};
use lightspeed_scheduler::job::Job;
use lightspeed_scheduler::scheduler::Scheduler;
use lightspeed_scheduler::JobExecutor;
use log::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ExtractorArgs::parse();

    // Initialize logger
    init_logger(&config.log_level)?;

    info!("Emvc Block Extractor");
    info!("----------------------");
    info!("- server_address: {}", config.server_address);
    info!("- remote_rpc_url: {}", config.remote_rpc_url);
    info!("- rpc_batch_size: {}", config.rpc_batch_size);
    info!("- request_time_out_secs: {}", config.request_time_out_secs);
    info!(
        "- reset_db_on_state_change: {}",
        config.reset_db_on_state_change
    );
    info!("----------------------");

    let evm_client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(config.remote_rpc_url.clone())));
    let db_client = config.command.clone().build_client().await?;

    // Start the job executor
    let job_executor = JobExecutor::new_with_local_tz();

    // Configure and start the block extractor task
    {
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
    }

    // Subscribe to the termination signals
    {
        let job_executor = job_executor.clone();
        tokio::spawn(async move {
            if let Err(err) = tokio::signal::ctrl_c().await {
                error!("unable to listen for shutdown signal: {err}");
            }
            if let Err(err) = job_executor.stop(true).await {
                error!("failed to shutdown the job executor gracefully: {err}");
            };
        });
    }

    // Start the job executor
    let job_executor_handle = job_executor.run().await?;

    // Start JSON RPC server
    let server = server_start(&config.server_address, db_client).await?;

    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            info!("Received shutdown signal");
        }
        Err(err) => error!("Failed to listen for shutdown signal: {err}"),
    }
    
    job_executor_handle
    .await
    .expect("error when running job executor");

    server_stop(server).await?;

    Ok(())
}

/// Initialize the logger
fn init_logger(logger_filter: &str) -> Result<(), SetLoggerError> {
    Builder::new().parse_filters(logger_filter).try_init()
}
