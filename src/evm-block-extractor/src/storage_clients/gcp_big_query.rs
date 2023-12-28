use super::BlockChainDB;
use ethers_core::types::{Block, Transaction};
//use gcp_bigquery_client::model::entry::Entry;
use gcp_bigquery_client::model::query_request::QueryRequest;
//use gcp_bigquery_client::model::row::Row;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use gcp_bigquery_client::Client;
use serde::Serialize;

const PROJECT_ID: &str = "bitfinity-evm";

/// A row in the BigQuery table
#[derive(Debug, Serialize)]
pub struct BlockRow {
    id: u64,
    body: serde_json::Value,
}

/// A client for BigQuery that can be used to query and insert data
pub struct BigQueryBlockChain {
    client: Client,
    project_id: String,
    dataset_id: String,
    table_id: String,
}

impl BigQueryBlockChain {
    pub async fn new(dataset_id: &str, table_id: &str) -> anyhow::Result<Self> {
        let sa_key = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .map_err(|_| anyhow::anyhow!("GOOGLE_APPLICATION_CREDENTIALS not set"))?;

        let client = Client::from_service_account_key_file(&sa_key).await?;

        Ok(Self {
            client,
            project_id: PROJECT_ID.to_string(),
            dataset_id: dataset_id.to_string(),
            table_id: table_id.to_string(),
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
            table_id = self.table_id,
        );
        let response = self
            .client
            .job()
            .query(self.project_id.as_str(), QueryRequest::new(query))
            .await?;

        let json = response.get_json_value(0)?;

        let block: Block<Transaction> = serde_json::from_value(json.unwrap())?;
        Ok(block)
    }

    /// Returns the number of the last block stored in the zip file
    async fn get_last_block_number(&self) -> anyhow::Result<u64> {
        let query = format!(
            "SELECT MAX(id) FROM `{project_id}.{dataset_id}.{table_id}`",
            project_id = self.project_id,
            dataset_id = self.dataset_id,
            table_id = self.table_id
        );
        let response = self
            .client
            .job()
            .query(self.project_id.as_str(), QueryRequest::new(query))
            .await?;

        let rows = response
            .get_i64_by_name("id")?
            .ok_or(anyhow::anyhow!("No id column in response"))?;

        Ok(rows as u64)
    }

    async fn insert_block(&mut self, block: Block<Transaction>) -> anyhow::Result<()> {
        let mut insert_request = TableDataInsertAllRequest::new();

        insert_request.add_row(
            None,
            BlockRow {
                id: block.number.unwrap().as_u64(),
                body: serde_json::to_value(block)?,
            },
        )?;

        self.client
            .tabledata()
            .insert_all(
                self.project_id.as_str(),
                self.dataset_id.as_str(),
                self.table_id.as_str(),
                insert_request,
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ethers_core::types::{Block, Transaction};
    use crate::storage_clients::gcp_big_query::{BigQueryBlockChain,BlockChainDB };

    #[tokio::test]
    async fn test_load_big_query_block_chain() {
        
        let sa_key_path = std::env::current_dir()
            .unwrap()
            .join("GCP_SA.json");
        println!(" Your path is ere {}", sa_key_path.display());

        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", sa_key_path.to_str().expect("Failed to convert path to string"));
        assert!(
            sa_key_path.exists(),
            "Service account key file does not exist"
        );
        let data_set_id = "testnet";
        let table_id = "blocks";

        let mut big_query = BigQueryBlockChain::new(data_set_id, table_id).await.unwrap();
        let mut test_block: Block<Transaction> = Block::default();
        test_block.number = Some(0u64.into());
        println!("{:?}", test_block);

        let _ = big_query.insert_block(test_block).await.unwrap();
    }
}
