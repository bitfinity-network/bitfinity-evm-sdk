use std::sync::Arc;

use alloy::eips::BlockNumberOrTag;
use alloy::primitives::{Address, U256, U64};
use did::evm_state::EvmGlobalState;
use did::{BlockConfirmationData, BlockConfirmationResult};
use ethereum_json_rpc_client::{Client, EthJsonRpcClient};
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::{ErrorCode, ErrorObject};

use crate::database::{CertifiedBlock, DatabaseClient};

#[derive(Clone)]
pub struct EthImpl<C>
where
    C: Client + Send + Sync + 'static,
{
    pub blockchain: Arc<dyn DatabaseClient + 'static>,
    pub evm_client: Arc<EthJsonRpcClient<C>>,
}

impl<C> EthImpl<C>
where
    C: Client + Send + Sync + 'static,
{
    pub fn new(
        db: Arc<dyn DatabaseClient + 'static>,
        evm_client: Arc<EthJsonRpcClient<C>>,
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

    #[method(name = "sendConfirmBlock")]
    async fn send_confirm_block(
        &self,
        data: BlockConfirmationData,
    ) -> RpcResult<BlockConfirmationResult>;
}

#[async_trait::async_trait]
impl<C> ICServer for EthImpl<C>
where
    C: Client + Send + Sync + 'static,
{
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
        self.evm_client.get_evm_global_state().await.map_err(|e| {
            log::error!("Error getting EVM global state: {:?}", e);
            ErrorObject::from(ErrorCode::InternalError)
        })
    }

    async fn send_confirm_block(
        &self,
        data: BlockConfirmationData,
    ) -> RpcResult<BlockConfirmationResult> {
        let block_info = self.blockchain.get_block_info().await.map_err(|e| {
            log::warn!("failed to get block info from database: {e}");
            ErrorCode::InternalError
        })?;

        let should_forward = match block_info {
            Some(info) if info.safe_block_number < data.block_number => true,
            None => true,
            _ => false,
        };

        let confirmation_result = if should_forward {
            self.evm_client
                .send_confirm_block(data)
                .await
                .map_err(|e| {
                    log::warn!("failed to send block confirmation to evm: {e}");
                    ErrorCode::InternalError
                })?
        } else {
            BlockConfirmationResult::AlreadyConfirmed
        };

        Ok(confirmation_result)
    }
}

#[async_trait::async_trait]
impl<C> EthServer for EthImpl<C>
where
    C: Client + Send + Sync + 'static,
{
    async fn get_block_by_number(
        &self,
        block: BlockNumberOrTag,
        include_transactions: bool,
    ) -> RpcResult<serde_json::Value> {
        let db = &self.blockchain;

        let block_info_future = async {
            match db.get_block_info().await {
                Ok(Some(info)) => Ok(info),
                Ok(None) => {
                    log::warn!("No block info set, can't select {block} block.");
                    Err(ErrorCode::InternalError)
                }
                Err(e) => {
                    log::warn!("Error getting blockchain block info: {:?}", e);
                    Err(ErrorCode::InternalError)
                }
            }
        };

        let Some(latest_block_in_db) =
            self.blockchain
                .get_latest_block_number()
                .await
                .map_err(|e| {
                    log::warn!("Error getting earliest block number: {:?}", e);
                    ErrorCode::InternalError
                })?
        else {
            return Ok(serde_json::Value::Null);
        };

        let block_number = match block {
            BlockNumberOrTag::Finalized => {
                let block_info = block_info_future.await?;
                block_info.finalized_block_number.min(latest_block_in_db)
            }
            BlockNumberOrTag::Safe => {
                let block_info = block_info_future.await?;
                block_info.safe_block_number.min(latest_block_in_db)
            }
            BlockNumberOrTag::Latest => latest_block_in_db,
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
