use std::fmt::Debug;

use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};
use gcp_bigquery_client::model::dataset::Dataset;
use gcp_bigquery_client::model::field_type::serialize_json_as_string;
use gcp_bigquery_client::model::query_parameter::QueryParameter;
use gcp_bigquery_client::model::query_parameter_type::QueryParameterType;
use gcp_bigquery_client::model::query_parameter_value::QueryParameterValue;
use gcp_bigquery_client::model::query_request::QueryRequest;
use gcp_bigquery_client::model::table::Table;
use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
use gcp_bigquery_client::model::table_data_insert_all_request_rows::TableDataInsertAllRequestRows;
use gcp_bigquery_client::model::table_field_schema::TableFieldSchema;
use gcp_bigquery_client::model::table_schema::TableSchema;
use gcp_bigquery_client::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::BlockChainDB;
use crate::constants::{BLOCKS_TABLE_ID, RECEIPTS_TABLE_ID};

#[derive(Clone)]
/// A client for BigQuery that can be used to query and insert data
pub struct BigQueryBlockChain {
    client: Client,
    /// The project ID of the BigQuery project
    project_id: String,
    /// The dataset ID of the BigQuery table
    ///
    /// Can be mainnet/testnet
    dataset_id: String,
}

impl BigQueryBlockChain {
    /// Creates a new BigQuery client
    pub async fn new(
        project_id: String,
        dataset_id: String,
        sa_key: String,
    ) -> anyhow::Result<Self> {
        let service_account = yup_oauth2::parse_service_account_key(sa_key)?;

        let client = Client::from_service_account_key(service_account, false).await?;

        Ok(Self {
            client,
            project_id,
            dataset_id,
        })
    }

    /// Creates a new BigQuery client with a custom client
    pub fn new_with_client(
        project_id: String,
        dataset_id: String,
        client: Client,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client,
            project_id,
            dataset_id,
        })
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let dataset = Dataset::new(&self.project_id, &self.dataset_id);
        // Make sure the dataset exists
        if self
            .client
            .dataset()
            .get(&self.project_id, &self.dataset_id)
            .await
            .is_err()
        {
            self.client.dataset().create(dataset.clone()).await?;
        }

        // Define tables with their respective schemas
        let tables = [
            (
                BLOCKS_TABLE_ID,
                vec![
                    TableFieldSchema::integer("id"),
                    TableFieldSchema::json("body"),
                ],
            ),
            (
                RECEIPTS_TABLE_ID,
                vec![
                    TableFieldSchema::string("tx_hash"),
                    TableFieldSchema::json("receipt"),
                ],
            ),
        ];

        // Check each table and create if it does not exist
        for (table_id, schema_fields) in &tables {
            let table_exists = self
                .client
                .table()
                .get(&self.project_id, &self.dataset_id, table_id, None)
                .await
                .is_ok();

            if !table_exists {
                dataset
                    .create_table(
                        &self.client,
                        Table::new(
                            &self.project_id,
                            &self.dataset_id,
                            table_id,
                            TableSchema::new(schema_fields.clone()),
                        ),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn execute_query<T: DeserializeOwned>(&self, query: QueryRequest) -> anyhow::Result<T> {
        let mut response = self.client.job().query(&self.project_id, query).await?;

        if response.next_row() {
            let result_str = response
                .get_string(0)?
                .ok_or(anyhow::anyhow!("Expected result not found in the response"))?
                .trim_matches('"')
                .replace("\\\"", "\"");

            let result: T = serde_json::from_str(&result_str)?;

            Ok(result)
        } else {
            Err(anyhow::anyhow!("No data found for the query"))
        }
    }

    async fn insert_batch_data(
        &self,
        table_id: &str,
        rows: Vec<TableDataInsertAllRequestRows>,
    ) -> anyhow::Result<()> {
        let mut insert_request = TableDataInsertAllRequest::new();

        insert_request.add_rows(rows)?;

        let res = self
            .client
            .tabledata()
            .insert_all(&self.project_id, &self.dataset_id, table_id, insert_request)
            .await?;

        if res.insert_errors.is_some() {
            println!("error inserting data: {:?}", res.insert_errors);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl BlockChainDB for BigQueryBlockChain {
    async fn get_block_by_number(&self, block_number: u64) -> anyhow::Result<Block<Transaction>> {
        let query_request = QueryRequest {
            query_parameters: Some(vec![QueryParameter {
                name: Some("id".to_string()),
                parameter_type: Some(QueryParameterType {
                    r#type: "INTEGER".to_string(),
                    ..Default::default()
                }),
                parameter_value: Some(QueryParameterValue {
                    value: Some(block_number.to_string()),
                    ..Default::default()
                }),
            }]),
            query: format!(
                "SELECT body FROM `{project_id}.{dataset_id}.{table_id}` WHERE id = @id",
                project_id = self.project_id,
                dataset_id = self.dataset_id,
                table_id = BLOCKS_TABLE_ID,
            ),
            ..Default::default()
        };

        self.execute_query(query_request).await
    }

    /// Returns the number of the last block stored in the zip file
    async fn get_blocks_in_range(&self, start: u64, end: u64) -> anyhow::Result<Vec<u64>> {
        let query_request = QueryRequest {
            query_parameters: Some(vec![
                QueryParameter {
                    name: Some("start".to_string()),
                    parameter_type: Some(QueryParameterType {
                        r#type: "INTEGER".to_string(),
                        ..Default::default()
                    }),
                    parameter_value: Some(QueryParameterValue {
                        value: Some(start.to_string()),
                        ..Default::default()
                    }),
                },
                QueryParameter {
                    name: Some("end".to_string()),
                    parameter_type: Some(QueryParameterType {
                        r#type: "INTEGER".to_string(),
                        ..Default::default()
                    }),
                    parameter_value: Some(QueryParameterValue {
                        value: Some(end.to_string()),
                        ..Default::default()
                    }),
                },
            ]),
            query: format!(
                "SELECT id FROM `{project_id}.{dataset_id}.{table_id}` WHERE id BETWEEN @start AND @end",
                project_id = self.project_id,
                dataset_id = self.dataset_id,
                table_id = BLOCKS_TABLE_ID,
            ),
            ..Default::default()
        };

        let mut response = self
            .client
            .job()
            .query(&self.project_id, query_request)
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

    async fn insert_blocks_and_receipts(
        &self,
        block: &[Block<Transaction>],
        receipts: &[TransactionReceipt],
    ) -> anyhow::Result<()> {
        let receipts = receipts
            .iter()
            .map(|r| {
                let tx_hash = r.transaction_hash;
                let receipt = ReceiptRow {
                    tx_hash: format!("0x{:x}", tx_hash),
                    receipt: serde_json::to_value(r).expect("Failed to serialize receipt"),
                };

                TableDataInsertAllRequestRows {
                    insert_id: Some(format!("0x{:x}", tx_hash)),
                    json: serde_json::to_value(receipt).expect("Failed to serialize receipt"),
                }
            })
            .collect::<Vec<_>>();

        self.insert_batch_data(RECEIPTS_TABLE_ID, receipts).await?;

        let blocks = block
            .iter()
            .map(|b| {
                let block_id = b
                    .number
                    .ok_or(anyhow::anyhow!("Block number not found"))
                    .expect("Block number not found")
                    .as_u64();

                let block_hash = b
                    .hash
                    .ok_or(anyhow::anyhow!("Block hash not found"))
                    .expect("Block hash not found");

                let block_row = BlockRow {
                    id: block_id,
                    body: serde_json::to_value(b).expect("Failed to serialize block"),
                };

                TableDataInsertAllRequestRows {
                    insert_id: Some(format!("0x{:x}", block_hash)),
                    json: serde_json::to_value(block_row).expect("Failed to serialize block"),
                }
            })
            .collect::<Vec<_>>();

        self.insert_batch_data(BLOCKS_TABLE_ID, blocks).await?;

        Ok(())
    }

    async fn get_transaction_receipt(&self, tx_hash: H256) -> anyhow::Result<TransactionReceipt> {
        let query_request = QueryRequest {
            query_parameters: Some(vec![
                QueryParameter {
                    name: Some("tx_hash".to_string()),
                    parameter_type: Some(QueryParameterType {
                        r#type: "STRING".to_string(),
                        ..Default::default()
                    }),
                    parameter_value: Some(QueryParameterValue {
                        value: Some(format!("0x{:x}", tx_hash)),
                        ..Default::default()
                    }),
                },
            ]),
            query:format!(
                "SELECT receipt FROM `{project_id}.{dataset_id}.{table_id}` WHERE tx_hash = @tx_hash",
                project_id = self.project_id,
                dataset_id = self.dataset_id,
                table_id = RECEIPTS_TABLE_ID,
            ),
            ..Default::default()
        };

        self.execute_query(query_request).await
    }

    async fn get_latest_block_number(&self) -> anyhow::Result<u64> {
        let query = format!(
            "SELECT MAX(id) as max_id FROM `{project_id}.{dataset_id}.{table_id}`",
            project_id = self.project_id,
            dataset_id = self.dataset_id,
            table_id = BLOCKS_TABLE_ID,
        );
        let mut response = self
            .client
            .job()
            .query(&self.project_id, QueryRequest::new(query))
            .await?;

        if response.next_row() {
            let max_id = response.get_i64(0)?.ok_or(anyhow::anyhow!(
                "Block with number {} not found in the database",
                0
            ))? as u64;

            Ok(max_id)
        } else {
            Err(anyhow::anyhow!(
                "Block with number {} not found in the database",
                0
            ))
        }
    }

    async fn get_earliest_block_number(&self) -> anyhow::Result<u64> {
        let query = format!(
            "SELECT MIN(id) as min_id FROM `{project_id}.{dataset_id}.{table_id}`",
            project_id = self.project_id,
            dataset_id = self.dataset_id,
            table_id = BLOCKS_TABLE_ID,
        );
        let mut response = self
            .client
            .job()
            .query(&self.project_id, QueryRequest::new(query))
            .await?;

        if response.next_row() {
            let min_id = response.get_i64(0)?.ok_or(anyhow::anyhow!(
                "Block with number {} not found in the database",
                0
            ))? as u64;

            Ok(min_id)
        } else {
            Err(anyhow::anyhow!(
                "Block with number {} not found in the database",
                0
            ))
        }
    }
}

/// A row in the BigQuery table
#[derive(Debug, Serialize)]
pub struct BlockRow {
    id: u64,
    #[serde(serialize_with = "serialize_json_as_string")]
    body: Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct ReceiptRow {
    tx_hash: String,
    #[serde(serialize_with = "serialize_json_as_string")]
    receipt: Value,
}
