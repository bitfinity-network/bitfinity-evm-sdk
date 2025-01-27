use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRpcClient;
use jsonrpsee::server::{Server, ServerHandle};
use jsonrpsee::RpcModule;
use log::*;

use crate::database::DatabaseClient;
use crate::rpc::{EthImpl, EthServer, ICServer};

/// Start the RPC server
pub async fn server_start(
    server_address: &str,
    db_client: Arc<dyn DatabaseClient>,
    evm_client: Option<Arc<EthJsonRpcClient<ReqwestClient>>>,
) -> anyhow::Result<ServerHandle> {
    info!("Start server");

    let server = Server::builder().build(server_address).await?;

    let eth = EthImpl::new(db_client, evm_client);

    let mut module = RpcModule::new(());

    module.merge(EthServer::into_rpc(eth.clone()))?;
    module.merge(ICServer::into_rpc(eth))?;

    info!("Server started on {}", server.local_addr()?);

    Ok(server.start(module))
}

/// Stop the RPC server
pub async fn server_stop(server: ServerHandle) -> anyhow::Result<()> {
    info!("Stopping server");
    server.stop()?;
    server.stopped().await;
    Ok(())
}
