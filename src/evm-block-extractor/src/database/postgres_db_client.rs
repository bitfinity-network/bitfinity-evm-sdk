use ::sqlx::migrate::Migrator;
use ::sqlx::*;
use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};
use serde::de::DeserializeOwned;
use sqlx::postgres::PgRow;

use super::DatabaseClient;

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
}

#[async_trait::async_trait]
impl DatabaseClient for PostgresDbClient {
    async fn init(&self) -> anyhow::Result<()> {
        MIGRATOR.run(&self.pool).await?;
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

        Ok(block.into_full_block(transactions))
    }

    async fn insert_block_data(
        &self,
        blocks: &[Block<H256>],
        receipts: &[TransactionReceipt],
        transactions: &[Transaction],
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        for block in blocks {
            let block_id = block
                .number
                .ok_or(anyhow::anyhow!("Block number not found"))?
                .as_u64();

            sqlx::query("INSERT INTO EVM_BLOCK (id, data) VALUES ($1, $2)")
                .bind(block_id as i64)
                .bind(serde_json::to_value(block)?)
                .execute(&mut *tx)
                .await
                .map_err(|e| anyhow::anyhow!("Error inserting block {}: {:?}", block_id, e))
                .map(|_| ())?;
        }

        for receipt in receipts {
            let hex_tx_hash = did::H256::from(receipt.transaction_hash).to_hex_str();
            sqlx::query("INSERT INTO EVM_TRANSACTION_RECEIPT (id, data) VALUES ($1, $2)")
                .bind(&hex_tx_hash)
                .bind(serde_json::to_value(receipt)?)
                .execute(&mut *tx)
                .await?;
        }

        for txn in transactions {
            let hex_tx_hash = did::H256::from(txn.hash).to_hex_str();
            sqlx::query("INSERT INTO EVM_TRANSACTION (id, data,block_number) VALUES ($1, $2,$3)")
                .bind(&hex_tx_hash)
                .bind(serde_json::to_value(txn)?)
                .bind(txn.block_number.expect("Block number not found").as_u64() as i64)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    /// Get a transaction receipt from the database
    async fn get_transaction_receipt(&self, tx_hash: H256) -> anyhow::Result<TransactionReceipt> {
        let hex_tx_hash = did::H256::from(tx_hash).to_hex_str();
        sqlx::query(
            "SELECT data FROM EVM_TRANSACTION_RECEIPT WHERE EVM_TRANSACTION_RECEIPT.id = $1",
        )
        .bind(&hex_tx_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Error getting transaction receipt {}: {:?}", hex_tx_hash, e))
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
