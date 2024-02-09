use std::fmt::Debug;

use did::transaction::StorableExecutionResult;
use did::{Block, Transaction, TransactionReceipt, H256};
use ethereum_json_rpc_client::http::HttpResponse;
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
use jsonrpc_core::Success;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::{AccountBalance, DataContainer, DatabaseClient, CHAIN_ID_KEY, GENESIS_BALANCES_KEY};

const BQ_EXE_RESULTS_TABLE_ID: &str = "exe_results";
const BQ_BLOCKS_TABLE_ID: &str = "blocks";
const BQ_CERTIFIED_BLOCKS_TABLE_ID: &str = "blocks";
const BQ_TRANSACTIONS_TABLE_ID: &str = "transactions";
const BQ_KEY_VALUE_TABLE_ID: &str = "key_value_data";

#[derive(Clone)]
/// A client for BigQuery that can be used to query and insert data
pub struct BigQueryDbClient {
    client: Client,
    /// The project ID of the BigQuery project
    project_id: String,
    /// The dataset ID of the BigQuery table
    ///
    /// Can be mainnet/testnet
    dataset_id: String,
}

impl BigQueryDbClient {
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

    async fn query_one<T: DeserializeOwned>(&self, query: QueryRequest) -> anyhow::Result<T> {
        self.query_one_optional(query)
            .await?
            .ok_or(anyhow::anyhow!("No data found for the query"))
    }

    async fn query_one_optional<T: DeserializeOwned>(
        &self,
        query: QueryRequest,
    ) -> anyhow::Result<Option<T>> {
        let mut response = self.client.job().query(&self.project_id, query).await?;

        if response.next_row() {
            let result_str = response
                .get_string(0)?
                .ok_or(anyhow::anyhow!("Expected result not found in the response"))?
                .trim_matches('"')
                .replace("\\\"", "\"");

            let result: T = serde_json::from_str(&result_str)?;

            Ok(Some(result))
        } else {
            Ok(None)
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
            log::error!("error inserting data: {:?}", res.insert_errors);
        }

        Ok(())
    }

    async fn create_tables_if_not_present(&self) -> anyhow::Result<()> {
        let dataset = Dataset::new(&self.project_id, &self.dataset_id);

        log::info!("Creating tables if not present");

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
                BQ_BLOCKS_TABLE_ID,
                vec![
                    TableFieldSchema::integer("id"),
                    TableFieldSchema::json("body"),
                ],
            ),
            (
                BQ_CERTIFIED_BLOCKS_TABLE_ID,
                vec![
                    TableFieldSchema::integer("id"),
                    TableFieldSchema::json("certified_response"),
                ],
            ),
            (
                BQ_EXE_RESULTS_TABLE_ID,
                vec![
                    TableFieldSchema::string("tx_hash"),
                    TableFieldSchema::json("exe_result"),
                ],
            ),
            (
                BQ_TRANSACTIONS_TABLE_ID,
                vec![
                    TableFieldSchema::string("tx_hash"),
                    TableFieldSchema::json("transaction"),
                    TableFieldSchema::integer("block_number"),
                ],
            ),
            (
                BQ_KEY_VALUE_TABLE_ID,
                vec![
                    TableFieldSchema::string("key"),
                    TableFieldSchema::json("data"),
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

    async fn fetch_key_value_data<D: DeserializeOwned>(
        &self,
        key: &str,
    ) -> anyhow::Result<Option<D>> {
        let query_request = QueryRequest {
            query_parameters: Some(vec![QueryParameter {
                name: Some("key".to_string()),
                parameter_type: Some(QueryParameterType {
                    r#type: "STRING".to_string(),
                    ..Default::default()
                }),
                parameter_value: Some(QueryParameterValue {
                    value: Some(key.to_string()),
                    ..Default::default()
                }),
            }]),
            query: format!(
                "SELECT data FROM `{project_id}.{dataset_id}.{table_id}` WHERE key = @key",
                project_id = self.project_id,
                dataset_id = self.dataset_id,
                table_id = BQ_KEY_VALUE_TABLE_ID,
            ),
            ..Default::default()
        };

        self.query_one_optional(query_request).await
    }

    async fn insert_key_value_data<D: Serialize>(&self, key: &str, data: D) -> anyhow::Result<()> {
        let json = KeyValueDataRow {
            key: key.to_string(),
            data: serde_json::to_value(data).expect("Failed to serialize data"),
        };

        let key_value_row = TableDataInsertAllRequestRows {
            insert_id: Some(key.to_string()),
            json: serde_json::to_value(json)?,
        };

        log::debug!("Inserting key value data with key [{}]", key);

        self.insert_batch_data(BQ_KEY_VALUE_TABLE_ID, vec![key_value_row])
            .await
    }
}

#[async_trait::async_trait]
impl DatabaseClient for BigQueryDbClient {
    async fn init(&self, block: Option<Block<H256>>, reset_database: bool) -> anyhow::Result<()> {
        self.create_tables_if_not_present().await?;

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
        log::warn!("BigQuery tables are being deleted");
        let delete_table = |table_id: String| async move {
            self.client
                .table()
                .delete(&self.project_id, &self.dataset_id, &table_id)
                .await
        };

        delete_table(BQ_BLOCKS_TABLE_ID.to_owned()).await?;
        delete_table(BQ_EXE_RESULTS_TABLE_ID.to_owned()).await?;
        delete_table(BQ_TRANSACTIONS_TABLE_ID.to_owned()).await?;
        delete_table(BQ_KEY_VALUE_TABLE_ID.to_owned()).await?;

        self.create_tables_if_not_present().await
    }

    async fn get_block_by_number(&self, block_number: u64) -> anyhow::Result<Block<H256>> {
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
                table_id = BQ_BLOCKS_TABLE_ID,
            ),
            ..Default::default()
        };

        self.query_one(query_request).await
    }

    async fn get_full_block_by_number(
        &self,
        block_number: u64,
    ) -> anyhow::Result<Block<Transaction>> {
        let block = self.get_block_by_number(block_number).await?;

        let query_request = QueryRequest {
            query_parameters: Some(vec![QueryParameter {
                name: Some("block_number".to_string()),
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
                "SELECT transaction FROM `{project_id}.{dataset_id}.{table_id}` WHERE block_number = @block_number",
                project_id = self.project_id,
                dataset_id = self.dataset_id,
                table_id = BQ_TRANSACTIONS_TABLE_ID,
            ),
            ..Default::default()
        };

        let mut transactions = vec![];

        let mut res = self
            .client
            .job()
            .query(&self.project_id, query_request)
            .await?;

        while res.next_row() {
            let res = res
                .get_string(0)?
                .ok_or(anyhow::anyhow!("Expected result not found in the response"))?
                .trim_matches('"')
                .replace("\\\"", "\"");

            let result: Transaction = serde_json::from_str(&res)?;

            transactions.push(result);
        }

        Ok(block.into_full_block(transactions))
    }

    async fn insert_block_data(
        &self,
        blocks: &[Block<H256>],
        receipts: &[StorableExecutionResult],
        transactions: &[Transaction],
    ) -> anyhow::Result<()> {
        if !blocks.is_empty() {
            log::info!(
                "Insert block data for blocks in range {} to {}",
                blocks[0].number,
                blocks[blocks.len() - 1].number
            );
        };

        if !receipts.is_empty() {
            let receipts = receipts
                .iter()
                .map(|r| {
                    let tx_hash = &r.transaction_hash;
                    let receipt = ExeResultRow {
                        tx_hash: format!("0x{:x}", tx_hash),
                        exe_result: serde_json::to_value(r).expect("Failed to serialize receipt"),
                    };

                    TableDataInsertAllRequestRows {
                        insert_id: Some(format!("0x{:x}", tx_hash)),
                        json: serde_json::to_value(receipt).expect("Failed to serialize receipt"),
                    }
                })
                .collect::<Vec<_>>();

            log::debug!("Inserting {} receipts", receipts.len());

            self.insert_batch_data(BQ_EXE_RESULTS_TABLE_ID, receipts)
                .await?;
        }

        if !blocks.is_empty() {
            let blocks = blocks
                .iter()
                .map(|b| {
                    let block_id = b.number.0.as_u64();

                    let block_row = BlockRow {
                        id: block_id,
                        body: serde_json::to_value(b).expect("Failed to serialize block"),
                    };

                    TableDataInsertAllRequestRows {
                        insert_id: Some(b.hash.to_hex_str()),
                        json: serde_json::to_value(block_row).expect("Failed to serialize block"),
                    }
                })
                .collect::<Vec<_>>();

            log::debug!("Inserting {} blocks", blocks.len());

            self.insert_batch_data(BQ_BLOCKS_TABLE_ID, blocks).await?;
        }

        if !transactions.is_empty() {
            let transactions = transactions
                .iter()
                .map(|txn| {
                    let tx_hash = &txn.hash;

                    let txn = TransactionRow {
                        tx_hash: format!("0x{:x}", tx_hash),
                        transaction: serde_json::to_value(txn)
                            .expect("Failed to serialize transaction"),
                        block_number: txn.block_number.expect("Block number not found").0.as_u64(),
                    };

                    TableDataInsertAllRequestRows {
                        insert_id: Some(format!("0x{:x}", tx_hash)),
                        json: serde_json::to_value(txn).expect("Failed to serialize transaction"),
                    }
                })
                .collect::<Vec<_>>();

            log::debug!("Inserting {} transactions", transactions.len());

            self.insert_batch_data(BQ_TRANSACTIONS_TABLE_ID, transactions)
                .await?;
        }

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
                "SELECT exe_result FROM `{project_id}.{dataset_id}.{table_id}` WHERE tx_hash = @tx_hash",
                project_id = self.project_id,
                dataset_id = self.dataset_id,
                table_id = BQ_EXE_RESULTS_TABLE_ID,
            ),
            ..Default::default()
        };

        let exe_result: StorableExecutionResult = self.query_one(query_request).await?;

        Ok(TransactionReceipt::from(exe_result))
    }

    async fn get_latest_block_number(&self) -> anyhow::Result<Option<u64>> {
        let query = format!(
            "SELECT MAX(id) as max_id FROM `{project_id}.{dataset_id}.{table_id}`",
            project_id = self.project_id,
            dataset_id = self.dataset_id,
            table_id = BQ_BLOCKS_TABLE_ID,
        );
        let mut response = self
            .client
            .job()
            .query(&self.project_id, QueryRequest::new(query))
            .await?;

        if response.next_row() {
            let max_id = response.get_i64(0)?;
            Ok(max_id.map(|v| v as u64))
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
            table_id = BQ_BLOCKS_TABLE_ID,
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

    async fn insert_certified_block_data(&self, response: HttpResponse) -> anyhow::Result<()> {
        let block = match serde_json::from_slice::<Success>(&response.body) {
            Ok(success) => serde_json::from_value::<Block<H256>>(success.result)?,
            Err(err) => anyhow::bail!("invalid response data: {err}"),
        };

        let block_row = BlockRow {
            id: block.number.0.as_u64(),
            body: serde_json::to_value(response).expect("Failed to serialize block"),
        };

        let rows = TableDataInsertAllRequestRows {
            insert_id: Some(block.hash.to_hex_str()),
            json: serde_json::to_value(block_row).expect("Failed to serialize block"),
        };

        self.insert_batch_data(BQ_CERTIFIED_BLOCKS_TABLE_ID, vec![rows])
            .await
    }

    async fn get_last_certified_block_data(&self) -> anyhow::Result<HttpResponse> {
        let query_request = QueryRequest {
            query_parameters: None,
            query: format!(
                "SELECT certified_response FROM `{project_id}.{dataset_id}.{table_id}` ORDER BY id DESC LIMIT 1",
                project_id = self.project_id,
                dataset_id = self.dataset_id,
                table_id = BQ_CERTIFIED_BLOCKS_TABLE_ID,
            ),
            ..Default::default()
        };

        let res: anyhow::Result<String> = self.query_one(query_request.clone()).await;
        eprintln!("get_last_certified_block_data result = {res:?}");

        self.query_one(query_request).await
    }
}

/// A row in the BigQuery table
#[derive(Debug, Serialize)]
pub struct BlockRow {
    id: u64,
    #[serde(serialize_with = "serialize_json_as_string")]
    body: Value,
}

#[derive(Debug, Serialize)]
pub struct CertifiedBlockRow {
    id: u64,
    #[serde(serialize_with = "serialize_json_as_string")]
    certified_response: Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExeResultRow {
    tx_hash: String,
    #[serde(serialize_with = "serialize_json_as_string")]
    exe_result: Value,
}

#[derive(Debug, Serialize, Clone)]
pub struct TransactionRow {
    tx_hash: String,
    #[serde(serialize_with = "serialize_json_as_string")]
    transaction: Value,
    block_number: u64,
}

/// A row in the BigQuery Key Value table
#[derive(Debug, Serialize)]
pub struct KeyValueDataRow {
    key: String,
    #[serde(serialize_with = "serialize_json_as_string")]
    data: Value,
}
