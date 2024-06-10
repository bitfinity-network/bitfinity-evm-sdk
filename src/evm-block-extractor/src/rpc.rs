use std::sync::Arc;

use ethers_core::types::{BlockNumber, H160, U256, U64};
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;

use crate::database::{CertifiedBlock, DatabaseClient};

#[derive(Clone)]
pub struct EthImpl {
    pub blockchain: Arc<dyn DatabaseClient + 'static>,
}

impl EthImpl {
    pub fn new(db: Arc<dyn DatabaseClient + 'static>) -> Self {
        Self { blockchain: db }
    }
}

/// eth_* RPC methods
#[rpc(server, namespace = "eth")]
pub trait Eth {
    #[method(name = "getBlockByNumber")]
    /// Get a block by number
    async fn get_block_by_number(
        &self,
        block: BlockNumber,
        full_transactions: bool,
    ) -> RpcResult<serde_json::Value>;

    #[method(name = "blockNumber")]
    /// Get the latest block number
    async fn block_number(&self) -> RpcResult<U256>;

    #[method(name = "chainId")]
    /// Get the chain id
    async fn get_chain_id(&self) -> RpcResult<U64>;
}

/// ic_* RPC methods
#[rpc(server, namespace = "ic")]
pub trait IC {
    #[method(name = "getGenesisBalances")]
    async fn get_genesis_balances(&self) -> RpcResult<Vec<(H160, U256)>>;

    #[method(name = "getLastCertifiedBlock")]
    async fn get_last_block_certified_data(&self) -> RpcResult<CertifiedBlock>;
}

#[async_trait::async_trait]
impl ICServer for EthImpl {
    async fn get_genesis_balances(&self) -> RpcResult<Vec<(H160, U256)>> {
        let tx = self.blockchain.get_genesis_balances().await.map_err(|e| {
            log::error!("Error getting genesis balances: {:?}", e);
            jsonrpsee::types::error::ErrorCode::InternalError
        })?;

        Ok(tx
            .unwrap_or_default()
            .into_iter()
            .map(|account| (account.address.into(), account.balance.into()))
            .collect())
    }

    async fn get_last_block_certified_data(&self) -> RpcResult<CertifiedBlock> {
        let certified_data = self
            .blockchain
            .get_last_certified_block_data()
            .await
            .map_err(|e| {
                log::error!("Error getting last block certified data: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?;

        Ok(certified_data)
    }
}

#[async_trait::async_trait]
impl EthServer for EthImpl {
    async fn get_block_by_number(
        &self,
        block: BlockNumber,
        include_transactions: bool,
    ) -> RpcResult<serde_json::Value> {
        let db = &self.blockchain;

        let block_number = match block {
            BlockNumber::Latest => db
                .get_latest_block_number()
                .await
                .map_err(|e| {
                    log::error!("Error getting block number: {:?}", e);
                    jsonrpsee::types::error::ErrorCode::InternalError
                })?
                .unwrap_or(0),
            BlockNumber::Earliest => db.get_earliest_block_number().await.map_err(|e| {
                log::error!("Error getting earliest block number: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?,
            BlockNumber::Number(num) => num.as_u64(),
            BlockNumber::Pending => return Ok(serde_json::Value::Null),
            _ => return Ok(serde_json::Value::Null),
        };

        if include_transactions {
            let block = self
                .blockchain
                .get_full_block_by_number(block_number)
                .await
                .map_err(|e| {
                    log::error!("Error getting block: {:?}", e);
                    jsonrpsee::types::error::ErrorCode::InternalError
                })?;

            let block = serde_json::to_value(&block).map_err(|e| {
                log::error!("Error serializing block: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?;

            Ok(block)
        } else {
            let block = self
                .blockchain
                .get_block_by_number(block_number)
                .await
                .map_err(|e| {
                    log::error!("Error getting block: {:?}", e);
                    jsonrpsee::types::error::ErrorCode::InternalError
                })?;

            let block = serde_json::to_value(&block).map_err(|e| {
                log::error!("Error serializing block: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?;

            Ok(block)
        }
    }

    async fn block_number(&self) -> RpcResult<U256> {
        let block_number = self
            .blockchain
            .get_latest_block_number()
            .await
            .map_err(|e| {
                log::error!("Error getting block number: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?
            .unwrap_or(0);

        Ok(block_number.into())
    }

    async fn get_chain_id(&self) -> RpcResult<U64> {
        let chain_id = self
            .blockchain
            .get_chain_id()
            .await
            .map_err(|e| {
                log::error!("Error getting chain id: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?
            .ok_or(jsonrpsee::types::error::ErrorCode::InternalError)?;

        Ok(chain_id.into())
    }
}
