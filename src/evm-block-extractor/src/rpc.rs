use std::sync::Arc;

use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{Address, U256, U64};
use did::evm_state::EvmGlobalState;
use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRpcClient;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::{ErrorCode, ErrorObject};

use crate::database::{CertifiedBlock, DatabaseClient};

#[derive(Clone)]
pub struct EthImpl {
    pub blockchain: Arc<dyn DatabaseClient + 'static>,
    pub evm_client: Option<Arc<EthJsonRpcClient<ReqwestClient>>>,
}

impl EthImpl {
    pub fn new(
        db: Arc<dyn DatabaseClient + 'static>,
        evm_client: Option<Arc<EthJsonRpcClient<ReqwestClient>>>,
    ) -> Self {
        Self {
            blockchain: db,
            evm_client,
        }
    }
}

/// eth_* RPC methods
#[rpc(server, namespace = "eth")]
pub trait Eth {
    #[method(name = "getBlockByNumber")]
    /// Get a block by number
    async fn get_block_by_number(
        &self,
        block: BlockNumberOrTag,
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
    async fn get_genesis_balances(&self) -> RpcResult<Vec<(Address, U256)>>;

    #[method(name = "getLastCertifiedBlock")]
    async fn get_last_block_certified_data(&self) -> RpcResult<CertifiedBlock>;

    #[method(name = "getEvmGlobalState")]
    async fn get_evm_global_state(&self) -> RpcResult<EvmGlobalState>;
}

#[async_trait::async_trait]
impl ICServer for EthImpl {
    async fn get_genesis_balances(&self) -> RpcResult<Vec<(Address, U256)>> {
        let tx = self.blockchain.get_genesis_balances().await.map_err(|e| {
            log::error!("Error getting genesis balances: {:?}", e);
            ErrorCode::InternalError
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
                ErrorCode::InternalError
            })?;

        Ok(certified_data)
    }

    async fn get_evm_global_state(&self) -> RpcResult<EvmGlobalState> {
        if let Some(evm_client) = &self.evm_client {
            evm_client.get_evm_global_state().await.map_err(|e| {
                log::error!("Error getting EVM global state: {:?}", e);
                ErrorObject::from(ErrorCode::InternalError)
            })
        } else {
            log::error!("EVM client not found");

            return Err(ErrorObject::owned(
                ErrorCode::InternalError.code(),
                "EVM client not found",
                None::<()>,
            ));
        }
    }
}

#[async_trait::async_trait]
impl EthServer for EthImpl {
    async fn get_block_by_number(
        &self,
        block: BlockNumberOrTag,
        include_transactions: bool,
    ) -> RpcResult<serde_json::Value> {
        let db = &self.blockchain;

        let block_number = match block {
            BlockNumberOrTag::Finalized | BlockNumberOrTag::Safe | BlockNumberOrTag::Latest => db
                .get_latest_block_number()
                .await
                .map_err(|e| {
                    log::error!("Error getting block number: {:?}", e);
                    ErrorCode::InternalError
                })?
                .unwrap_or(0),
            BlockNumberOrTag::Earliest => db.get_earliest_block_number().await.map_err(|e| {
                log::error!("Error getting earliest block number: {:?}", e);
                ErrorCode::InternalError
            })?,
            BlockNumberOrTag::Number(num) => num,
            BlockNumberOrTag::Pending => return Ok(serde_json::Value::Null),
        };

        if include_transactions {
            let block = self
                .blockchain
                .get_full_block_by_number(block_number)
                .await
                .map_err(|e| {
                    log::error!("Error getting block: {:?}", e);
                    ErrorCode::InternalError
                })?;

            let block = serde_json::to_value(&block).map_err(|e| {
                log::error!("Error serializing block: {:?}", e);
                ErrorCode::InternalError
            })?;

            Ok(block)
        } else {
            let block = self
                .blockchain
                .get_block_by_number(block_number)
                .await
                .map_err(|e| {
                    log::error!("Error getting block: {:?}", e);
                    ErrorCode::InternalError
                })?;

            let block = serde_json::to_value(&block).map_err(|e| {
                log::error!("Error serializing block: {:?}", e);
                ErrorCode::InternalError
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
                ErrorCode::InternalError
            })?
            .unwrap_or(0);

        Ok(U256::from(block_number))
    }

    async fn get_chain_id(&self) -> RpcResult<U64> {
        let chain_id = self
            .blockchain
            .get_chain_id()
            .await
            .map_err(|e| {
                log::error!("Error getting chain id: {:?}", e);
                ErrorCode::InternalError
            })?
            .ok_or(ErrorCode::InternalError)?;

        Ok(U64::from(chain_id))
    }
}
