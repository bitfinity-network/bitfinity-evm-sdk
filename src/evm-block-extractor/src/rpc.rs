use std::sync::Arc;

use did::TransactionReceipt;
use ethereum_json_rpc_client::http::HttpResponse;
use ethers_core::types::{BlockNumber, H160, H256, U256, U64};
use ethers_core::utils::rlp::{RlpStream, EMPTY_LIST_RLP};
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;

use crate::database::DatabaseClient;

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
        block_number: U256,
        full_transactions: bool,
    ) -> RpcResult<serde_json::Value>;

    #[method(name = "getTransactionReceipt")]
    /// Get a transaction receipt
    async fn get_transaction_receipt(&self, tx_hash: H256) -> RpcResult<TransactionReceipt>;

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
    #[method(name = "getBlocksRLP")]
    async fn get_blocks_rlp(&self, from: BlockNumber, max_number: U64) -> RpcResult<String>;

    #[method(name = "getGenesisBalances")]
    async fn get_genesis_balances(&self) -> RpcResult<Vec<(H160, U256)>>;

    #[method(name = "getLastBlockCertifiedData")]
    async fn get_last_block_certified_data(&self) -> RpcResult<HttpResponse>;
}

#[async_trait::async_trait]
impl ICServer for EthImpl {
    async fn get_blocks_rlp(&self, from: BlockNumber, max_number: U64) -> RpcResult<String> {
        let db = &self.blockchain;
        let from = match from {
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
            BlockNumber::Pending => return Ok(hex::encode(EMPTY_LIST_RLP)),
            _ => return Ok(hex::encode(EMPTY_LIST_RLP)),
        };

        let block_count = db
            .get_latest_block_number()
            .await
            .map_err(|e| {
                log::error!("Error getting block number: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?
            .unwrap_or(0)
            + 1;

        let end_block = std::cmp::min(from + std::cmp::min(10, max_number.as_u64()), block_count);

        if end_block <= from {
            return Ok(hex::encode(EMPTY_LIST_RLP));
        }

        let mut rlp = RlpStream::new_list((end_block - from) as usize);
        for block_index in from..end_block {
            let block = db
                .get_full_block_by_number(block_index)
                .await
                .map_err(|e| {
                    log::error!("Error getting block: {:?}", e);
                    jsonrpsee::types::error::ErrorCode::InternalError
                })?;

            rlp.append(&block);
        }

        Ok(hex::encode(rlp.out()))
    }

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

    async fn get_last_block_certified_data(&self) -> RpcResult<HttpResponse> {
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
        block_number: U256,
        include_transactions: bool,
    ) -> RpcResult<serde_json::Value> {
        let block_number = block_number.as_u64();

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

    async fn get_transaction_receipt(&self, tx_hash: H256) -> RpcResult<TransactionReceipt> {
        let tx = self
            .blockchain
            .get_transaction_receipt(tx_hash.into())
            .await
            .map_err(|e| {
                log::error!("Error getting transaction receipt: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?;

        Ok(tx)
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
