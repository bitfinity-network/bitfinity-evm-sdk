use ::sqlx::migrate::Migrator;
use ::sqlx::*;
use did::{Block, Transaction, H256};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::postgres::PgRow;

use super::{AccountBalance, DataContainer, DatabaseClient, CHAIN_ID_KEY, GENESIS_BALANCES_KEY};

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
        sqlx::query("TRUNCATE TABLE EVM_BLOCK, EVM_TRANSACTION, EVM_KEY_VALUE_DATA")
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
            let block_id = block.number.0.as_u64();

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
                .bind(txn.block_number.expect("Block number not found").0.as_u64() as i64)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
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
