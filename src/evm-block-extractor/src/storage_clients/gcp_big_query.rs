use ethers_core::types::{Block, Transaction, TransactionReceipt};
use gcp_bigquery_client::model::query_request::QueryRequest;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use gcp_bigquery_client::model::table_data_insert_all_request_rows::TableDataInsertAllRequestRows;
use gcp_bigquery_client::Client;
use serde::Serialize;

use super::BlockChainDB;
use crate::constants::{BLOCKS_TABLE_ID, PROJECT_ID, RECEIPTS_TABLE_ID};

/// A row in the BigQuery table
#[derive(Debug, Serialize)]
pub struct BlockRow {
    id: u64,
    body: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ReceiptRow {
    tx_hash: String,
    body: String,
}

#[derive(Clone)]
/// A client for BigQuery that can be used to query and insert data
pub struct BigQueryBlockChain {
    client: Client,
    project_id: String,
    dataset_id: String,
}

impl BigQueryBlockChain {
    pub async fn new(dataset_id: String) -> anyhow::Result<Self> {
        let sa_key = std::env::var("GCP_BLOCK_EXTRACTOR_SA_KEY")
            .map_err(|_| anyhow::anyhow!("GCP_BLOCK_EXTRACTOR_SA_KEY not set"))?;

        let client = Client::from_service_account_key_file(&sa_key).await?;

        Ok(Self {
            client,
            project_id: PROJECT_ID.to_string(),
            dataset_id: dataset_id.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl BlockChainDB for BigQueryBlockChain {
    /// Creates a new client for BigQuery
    async fn get_block_by_number(&self, block_number: u64) -> anyhow::Result<Block<Transaction>> {
        let query = format!(
            "SELECT body FROM `{project_id}.{dataset_id}.{table_id}` WHERE id = {block_number}",
            project_id = self.project_id,
            dataset_id = self.dataset_id,
            table_id = BLOCKS_TABLE_ID,
        );
        let response = self
            .client
            .job()
            .query(self.project_id.as_str(), QueryRequest::new(query))
            .await?;

        let json = response.get_json_value(0)?.ok_or(anyhow::anyhow!(
            "Block with number {} not found in the database",
            block_number
        ))?;

        let block: Block<Transaction> = serde_json::from_value(json)?;
        Ok(block)
    }

    /// Returns the number of the last block stored in the zip file
    async fn get_blocks_in_range(&self, start: u64, end: u64) -> anyhow::Result<Vec<u64>> {
        let query = format!(
            "SELECT id FROM `{project_id}.{dataset_id}.{table_id}` WHERE id BETWEEN {start} AND {end}",
            project_id = self.project_id,
            dataset_id = self.dataset_id,
            table_id = BLOCKS_TABLE_ID,

        );
        let mut response = self
            .client
            .job()
            .query(self.project_id.as_str(), QueryRequest::new(query))
            .await?;

        let mut rows: Vec<u64> = vec![];

        while response.next_row() {
            let name = response.get_i64_by_name("id")?.ok_or(anyhow::anyhow!(
                "Block with number {} not found in the database",
                start
            ))? as u64;

            rows.push(name)
        }

        Ok(rows)
    }

    async fn insert_block(&mut self, block: &Block<Transaction>) -> anyhow::Result<()> {
        let mut insert_request = TableDataInsertAllRequest::new();

        let block_id = block.number.unwrap().as_u64();
        let block_row = BlockRow {
            id: block_id,
            body: serde_json::to_string(&block)?,
        };

        // Check if block id already exists in the database
        let existing_blocks = self.get_blocks_in_range(block_id, block_id).await?;
        if existing_blocks.contains(&block_id) {
            return Err(anyhow::anyhow!("Block with id {} already exists", block_id));
        }

        insert_request.add_row(Some(block_id.to_string()), block_row)?;

        let res = self
            .client
            .tabledata()
            .insert_all(
                self.project_id.as_str(),
                self.dataset_id.as_str(),
                BLOCKS_TABLE_ID,
                insert_request,
            )
            .await?;

        if res.insert_errors.is_some() {
            println!("error inserting block: {:?}", res.insert_errors);
        }

        Ok(())
    }

    async fn insert_receipts(&mut self, receipts: &[TransactionReceipt]) -> anyhow::Result<()> {
        let mut insert_request = TableDataInsertAllRequest::new();

        let txes = receipts
            .iter()
            .map(|r| TableDataInsertAllRequestRows {
                insert_id: Some(r.transaction_hash.to_string()),
                json: serde_json::to_value(r).expect("Failed to serialize receipt"),
            })
            .collect::<Vec<_>>();

        insert_request.add_rows(txes)?;

        let res = self
            .client
            .tabledata()
            .insert_all(
                self.project_id.as_str(),
                self.dataset_id.as_str(),
                RECEIPTS_TABLE_ID,
                insert_request,
            )
            .await?;

        if res.insert_errors.is_some() {
            println!("error inserting receipt: {:?}", res.insert_errors);
        }

        Ok(())
    }

    async fn get_transaction_receipt(&self, tx_hash: String) -> anyhow::Result<TransactionReceipt> {
        let query = format!(
            "SELECT body FROM `{project_id}.{dataset_id}.{table_id}` WHERE tx_hash = '{tx_hash}'",
            project_id = self.project_id,
            dataset_id = self.dataset_id,
            table_id = RECEIPTS_TABLE_ID,
            tx_hash = tx_hash
        );
        let response = self
            .client
            .job()
            .query(self.project_id.as_str(), QueryRequest::new(query))
            .await?;

        let json = response.get_json_value(0)?.ok_or(anyhow::anyhow!(
            "Receipt with tx_hash {} not found in the database",
            tx_hash
        ))?;

        let receipt: TransactionReceipt = serde_json::from_value(json)?;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use ethers_core::types::{Block, Transaction};

    use crate::storage_clients::gcp_big_query::{BigQueryBlockChain, BlockChainDB};

    #[tokio::test]
    async fn test_load_big_query_block_chain() {
        let sa_key_path = std::env::current_dir().unwrap().join("GCP_SA.json");
        println!(" Your path is here {}", sa_key_path.display());

        std::env::set_var(
            "GCP_BLOCK_EXTRACTOR_SA_KEY",
            sa_key_path
                .to_str()
                .expect("Failed to convert path to string"),
        );
        assert!(
            sa_key_path.exists(),
            "Service account key file does not exist"
        );
        let data_set_id = "testnet";

        let mut big_query = BigQueryBlockChain::new(data_set_id.to_string())
            .await
            .unwrap();

        let test_block: Block<Transaction> = Block {
            number: Some(0u64.into()),
            ..Default::default()
        };

        //TODO: refactor with enum
        match big_query.insert_block(&test_block).await {
            Ok(_) => (),
            Err(e) => {
                if e.to_string().contains("Block with id") {
                    println!("Ignoring error: {}", e);
                } else {
                    panic!("Unhandled error: {}", e);
                }
            }
        }
        let blocks_in_range = big_query.get_blocks_in_range(0, 1).await.unwrap();
        println!("{:?}", blocks_in_range);
        assert_eq!(blocks_in_range[0], 0);
    }
}
