use ethers_core::types::{Block, BlockNumber, Transaction, TransactionReceipt, H256, U256, U64};
use ethers_core::utils::rlp::{RlpStream, EMPTY_LIST_RLP};
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;

use crate::storage_clients::BlockChainDB;

#[derive(Clone)]
pub struct EthImpl<B: BlockChainDB + 'static> {
    pub blockchain: B,
}

impl<B: BlockChainDB + 'static> EthImpl<B> {
    pub fn new(db: B) -> Self {
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
    ) -> RpcResult<Block<Transaction>>;

    #[method(name = "getTransactionReceipt")]
    /// Get a transaction receipt
    async fn get_transaction_receipt(&self, tx_hash: H256) -> RpcResult<TransactionReceipt>;

    #[method(name = "blockNumber")]
    /// Get the latest block number
    async fn block_number(&self) -> RpcResult<U256>;
}

/// ic_* RPC methods
#[rpc(server, namespace = "ic")]
pub trait IC {
    #[method(name = "getBlocksRLP")]
    async fn get_blocks_rlp(&self, from: BlockNumber, max_number: U64) -> RpcResult<Vec<u8>>;
}

#[async_trait::async_trait]
impl<B: BlockChainDB + 'static> ICServer for EthImpl<B> {
    async fn get_blocks_rlp(&self, from: BlockNumber, max_number: U64) -> RpcResult<Vec<u8>> {
        let db = &self.blockchain;
        let from = match from {
            BlockNumber::Latest => db.get_latest_block_number().await.map_err(|e| {
                log::error!("Error getting block number: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?.unwrap_or(0),
            BlockNumber::Earliest => db.get_earliest_block_number().await.map_err(|e| {
                log::error!("Error getting earliest block number: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?,
            BlockNumber::Number(num) => num.as_u64(),
            BlockNumber::Pending => return Ok(EMPTY_LIST_RLP.into()),
            _ => return Ok(EMPTY_LIST_RLP.into()),
        };

        let block_count = db.get_latest_block_number().await.map_err(|e| {
            log::error!("Error getting block number: {:?}", e);
            jsonrpsee::types::error::ErrorCode::InternalError
        })?.unwrap_or(0) + 1;

        let end_block = std::cmp::min(from + std::cmp::min(10, max_number.as_u64()), block_count);

        if end_block <= from {
            return Ok(EMPTY_LIST_RLP.into());
        }

        let mut rlp = RlpStream::new_list(Into::<u64>::into(end_block - from).try_into().unwrap());
        for block_index in from..end_block {
            let block: did::Block<did::Transaction> = db
                .get_block_by_number(block_index)
                .await
                .map_err(|e| {
                    log::error!("Error getting block: {:?}", e);
                    jsonrpsee::types::error::ErrorCode::InternalError
                })?
                .into();

            rlp.append(&block);
        }

        Ok(rlp.out().to_vec())
    }
}

#[async_trait::async_trait]
impl<B: BlockChainDB + 'static> EthServer for EthImpl<B> {
    async fn get_block_by_number(
        &self,
        block_number: U256,
        _full_transaction: bool,
    ) -> RpcResult<Block<Transaction>> {
        let block_number = block_number.as_u64();

        let block = self
            .blockchain
            .get_block_by_number(block_number)
            .await
            .map_err(|e| {
                log::error!("Error getting block: {:?}", e);
                jsonrpsee::types::error::ErrorCode::InternalError
            })?;

        Ok(block)
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> RpcResult<TransactionReceipt> {
        let tx = self
            .blockchain
            .get_transaction_receipt(tx_hash)
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
}
