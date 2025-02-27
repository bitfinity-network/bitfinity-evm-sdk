use std::collections::HashMap;

use ::sqlx::migrate::Migrator;
use ::sqlx::*;
use did::{Block, BlockchainBlockInfo, Transaction, H256};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::postgres::PgRow;

use super::{
    AccountBalance, CertifiedBlock, DataContainer, DatabaseClient, DiscardedBlock,
    BLOCKCHAIN_BLOCK_INFO_KEY, CHAIN_ID_KEY, GENESIS_BALANCES_KEY,
};

static MIGRATOR: Migrator = ::sqlx::migrate!("src_resources/db/postgres/migrations");

/// A blockchain client for Postgres
#[derive(Clone)]
pub struct PostgresDbClient {
    pool: PgPool,
}

impl PostgresDbClient {
    /// Create a new Postgres blockchain client
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn fetch_key_value_data<D: DeserializeOwned>(
        &self,
        key: &str,
    ) -> anyhow::Result<Option<D>> {
        let row = sqlx::query("SELECT data FROM EVM_KEY_VALUE_DATA WHERE KEY = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Error getting value data for key {}: {:?}", key, e))?;

        if let Some(row) = row {
            from_row_value(&row, 0).map(Some)
        } else {
            Ok(None)
        }
    }

    async fn insert_key_value_data<D: Serialize>(&self, key: &str, data: D) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO EVM_KEY_VALUE_DATA (key, data) VALUES ($1, $2)")
            .bind(key)
            .bind(serde_json::to_value(data)?)
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Error inserting value data for key {}: {:?}", key, e))
            .map(|_| ())
    }

    async fn override_key_value_data<D: Serialize>(
        &self,
        key: &str,
        data: D,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO EVM_KEY_VALUE_DATA (key, data) VALUES ($1, $2)
            ON CONFLICT (key) DO UPDATE SET data = $2",
        )
        .bind(key)
        .bind(serde_json::to_value(data)?)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Error inserting value data for key {}: {:?}", key, e))
        .map(|_| ())
    }
}

#[async_trait::async_trait]
impl DatabaseClient for PostgresDbClient {
    async fn init(&self, block: Option<Block<H256>>, reset_database: bool) -> anyhow::Result<()> {
        MIGRATOR.run(&self.pool).await?;

        if let Some(_latest_block_number) = self.get_latest_block_number().await? {
            if let Some(block) = block {
                if !self.check_if_same_block_hash(&block).await? {
                    if reset_database {
                        self.clear().await?;
                    } else {
                        return Err(anyhow::anyhow!(
                            "The block hash in the database is different from the one in the block"
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn clear(&self) -> anyhow::Result<()> {
        log::warn!("Postgres tables are being cleared");
        sqlx::query(
            "TRUNCATE TABLE EVM_BLOCK, EVM_TRANSACTION, EVM_KEY_VALUE_DATA, CERTIFIED_EVM_BLOCK",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_block_by_number(&self, block: u64) -> anyhow::Result<Block<H256>> {
        sqlx::query("SELECT data FROM EVM_BLOCK WHERE EVM_BLOCK.id = $1")
            .bind(block as i64)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Error getting block {}: {:?}", block, e))
            .and_then(|row| from_row_value(&row, 0))
    }

    async fn get_full_block_by_number(
        &self,
        block_number: u64,
    ) -> anyhow::Result<Block<Transaction>> {
        let block = self.get_block_by_number(block_number).await?;

        let transactions: Vec<Transaction> =
            sqlx::query("SELECT data FROM EVM_TRANSACTION WHERE EVM_TRANSACTION.block_number = $1")
                .bind(block_number as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Error getting transactions for block {:?}: {:?}", block, e)
                })
                .and_then(|row| from_rows_value(&row, 0))?;

        Ok(block.into_full_block(transactions)?)
    }

    async fn insert_block_data(
        &self,
        blocks: &[Block<H256>],
        transactions: &[Transaction],
    ) -> anyhow::Result<()> {
        if !blocks.is_empty() {
            log::info!(
                "Insert block data for blocks in range {} to {}",
                blocks[0].number,
                blocks[blocks.len() - 1].number
            );
        };

        let mut tx = self.pool.begin().await?;

        for block in blocks {
            let block_id = block.number.0.to::<u64>();

            sqlx::query("INSERT INTO EVM_BLOCK (id, data) VALUES ($1, $2)")
                .bind(block_id as i64)
                .bind(serde_json::to_value(block)?)
                .execute(&mut *tx)
                .await
                .map_err(|e| anyhow::anyhow!("Error inserting block {}: {:?}", block_id, e))
                .map(|_| ())?;
        }

        for txn in transactions {
            let hex_tx_hash = txn.hash.to_hex_str();
            sqlx::query("INSERT INTO EVM_TRANSACTION (id, data, block_number) VALUES ($1, $2,$3)")
                .bind(&hex_tx_hash)
                .bind(serde_json::to_value(txn)?)
                .bind(
                    txn.block_number
                        .expect("Block number not found")
                        .0
                        .to::<u64>() as i64,
                )
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    async fn insert_certified_block_data(&self, response: CertifiedBlock) -> anyhow::Result<()> {
        let block_id = response.data.number.0.to::<u64>();

        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO CERTIFIED_EVM_BLOCK (id, certified_response) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET certified_response = $2")
                    .bind(block_id as i64)
                    .bind(serde_json::to_value(response)?)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| anyhow::anyhow!("Error inserting certified block {}: {:?}", block_id, e))
                    .map(|_| ())?;
        tx.commit().await?;

        Ok(())
    }

    async fn get_last_certified_block_data(&self) -> anyhow::Result<CertifiedBlock> {
        sqlx::query("SELECT certified_response FROM CERTIFIED_EVM_BLOCK ORDER BY id DESC LIMIT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Error getting last certified block: {:?}", e))
            .and_then(|row| from_row_value(&row, 0))
    }

    /// Get the latest block number
    async fn get_latest_block_number(&self) -> anyhow::Result<Option<u64>> {
        sqlx::query("SELECT MAX(id) FROM EVM_BLOCK")
            .fetch_one(&self.pool)
            .await
            .map(|row| {
                row.try_get::<i64, _>(0)
                    .map(|n| Some(n as u64))
                    .unwrap_or(None)
            })
            .map_err(|e| anyhow::anyhow!("Error getting latest block number: {:?}", e))
    }

    /// Get earliest block number
    async fn get_earliest_block_number(&self) -> anyhow::Result<u64> {
        sqlx::query("SELECT MIN(id) FROM EVM_BLOCK")
            .fetch_one(&self.pool)
            .await
            .and_then(|row| row.try_get::<i64, _>(0).map(|n| n as u64))
            .map_err(|e| anyhow::anyhow!("Error getting earliest block number: {:?}", e))
    }

    async fn get_genesis_balances(&self) -> anyhow::Result<Option<Vec<AccountBalance>>> {
        self.fetch_key_value_data(GENESIS_BALANCES_KEY).await
    }

    async fn insert_genesis_balances(
        &self,
        genesis_balances: &[AccountBalance],
    ) -> anyhow::Result<()> {
        self.insert_key_value_data(GENESIS_BALANCES_KEY, genesis_balances)
            .await
    }

    async fn get_chain_id(&self) -> anyhow::Result<Option<u64>> {
        let data: Option<DataContainer<u64>> = self.fetch_key_value_data(CHAIN_ID_KEY).await?;
        Ok(data.map(|d| d.data))
    }

    async fn insert_chain_id(&self, chain_id: u64) -> anyhow::Result<()> {
        self.insert_key_value_data(CHAIN_ID_KEY, DataContainer::new(chain_id))
            .await
    }

    async fn get_transaction(&self, tx_hash: H256) -> anyhow::Result<Transaction> {
        let hex_tx_hash = did::H256::from(tx_hash).to_hex_str();
        sqlx::query("SELECT data FROM EVM_TRANSACTION WHERE id = $1")
            .bind(&hex_tx_hash)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Error getting transaction {}: {:?}", hex_tx_hash, e))
            .and_then(|row| from_row_value(&row, 0))
    }

    async fn discard_blocks_from(&self, start_from: u64, reason: &str) -> anyhow::Result<()> {
        log::warn!("Discarding blocks starting with {start_from}");

        let mut tx = self.pool.begin().await?;

        let block_rows = sqlx::query("DELETE FROM evm_block WHERE id >= $1 RETURNING data")
            .bind(start_from as i64)
            .bind(reason)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to discard tail: {e}"))?;

        let blocks_with_hashes = from_rows_value::<did::Block<did::H256>>(&block_rows, 0)?;

        let tx_rows =
            sqlx::query("DELETE FROM evm_transaction WHERE block_number >= $1 RETURNING data")
                .bind(start_from as i64)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to discard tail: {e}"))?;

        let tx_by_hash: HashMap<_, _> = tx_rows
            .into_iter()
            .filter_map(|r| {
                let tx = from_row_value::<did::Transaction>(&r, 0)
                    .inspect_err(|e| {
                        log::warn!("failed to decode tx data while discardirding: {e}");
                    })
                    .ok()?;
                Some((tx.hash.clone(), tx))
            })
            .collect();

        let full_blocks = blocks_with_hashes.into_iter().filter_map(|b| {
            let txs = b
                .transactions
                .iter()
                .filter_map(|h| tx_by_hash.get(h).cloned())
                .collect();
            b.into_full_block(txs)
                .inspect_err(|e| {
                    log::warn!("failed to build full block from txs while discarding: {e}")
                })
                .ok()
        });

        for block in full_blocks {
            let block_hash_str = block.hash.to_hex_str();

            sqlx::query("INSERT INTO DISCARDED_EVM_BLOCK (id, data, reason) VALUES ($1, $2, $3)")
                .bind(&block_hash_str)
                .bind(serde_json::to_value(block)?)
                .bind(reason)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Error inserting discarded block {}: {:?}",
                        block_hash_str,
                        e
                    )
                })
                .map(|_| ())?;
        }

        sqlx::query("DELETE FROM certified_evm_block WHERE id >= $1")
            .bind(start_from as i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to discard tail: {e}"))?;

        tx.commit().await?;

        Ok(())
    }

    async fn get_discarded_block_by_hash(&self, hash: H256) -> anyhow::Result<DiscardedBlock> {
        let hash_str = hash.to_hex_str();
        sqlx::query(
            "SELECT data, reason, discarded_at FROM DISCARDED_EVM_BLOCK WHERE DISCARDED_EVM_BLOCK.id = $1",
        )
        .bind(&hash_str)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Error getting discarded block {}: {:?}", hash_str, e))
        .and_then(|row| {
            Ok(DiscardedBlock {
                block: from_row_value(&row, 0)?,
                reason: row.try_get(1)?,
                timestamp: row.try_get(2)?,
            })
        })
    }

    async fn get_block_info(&self) -> anyhow::Result<Option<BlockchainBlockInfo>> {
        self.fetch_key_value_data(BLOCKCHAIN_BLOCK_INFO_KEY).await
    }

    async fn set_block_info(&self, info: BlockchainBlockInfo) -> anyhow::Result<()> {
        self.override_key_value_data(BLOCKCHAIN_BLOCK_INFO_KEY, info)
            .await
    }
}

fn from_row_value<T: DeserializeOwned>(row: &PgRow, index: usize) -> anyhow::Result<T> {
    let res = serde_json::from_value(row.try_get::<serde_json::Value, _>(index)?)?;
    Ok(res)
}

fn from_rows_value<T: DeserializeOwned>(rows: &[PgRow], index: usize) -> anyhow::Result<Vec<T>> {
    let mut res = Vec::with_capacity(rows.len());
    for row in rows {
        res.push(from_row_value(row, index)?);
    }
    Ok(res)
}
