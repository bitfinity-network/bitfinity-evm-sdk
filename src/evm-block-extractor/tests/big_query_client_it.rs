//! Run BigQuery locally by using [big-query-emulator](https://github.com/goccy/bigquery-emulator).
//!
//! With Docker:
//! ```bash
//! docker run -p 9050:9050 ghcr.io/goccy/bigquery-emulator:latest --project=my_project
//! ```

mod client;
use bq::BQ;
use ethers_core::types::{Block, Transaction, TransactionReceipt, H256};
use evm_block_extractor::storage_clients::gcp_big_query::BigQueryBlockChain;
use evm_block_extractor::storage_clients::BlockChainDB;
use testcontainers::clients::Cli;

mod bq {

    // use fake::{Fake, StringFaker};
    use gcp_bigquery_client::model::dataset::Dataset;
    use gcp_bigquery_client::model::query_request::QueryRequest;
    use gcp_bigquery_client::model::table::Table;
    use gcp_bigquery_client::model::table_data_insert_all_request::TableDataInsertAllRequest;
    use gcp_bigquery_client::model::table_field_schema::TableFieldSchema;
    use gcp_bigquery_client::model::table_schema::TableSchema;
    use gcp_bigquery_client::Client;
    use serde::Serialize;

    // The project ID needs to match with the flag `--project` of the bigquery emulator.
    const NAME_COLUMN: &str = "name";
    const TABLE_ID: &str = "table";

    pub struct BQ {
        client: Client,
        project_id: String,
        dataset_id: String,
        table_id: String,
    }

    #[derive(Serialize, Debug, Clone, PartialEq, Eq)]
    pub struct Row {
        pub name: String,
    }

    impl BQ {
        pub async fn new(client: Client, project_id: &str) -> Self {
            // Use a random dataset id, so that each run is isolated.
            // let dataset_id: String = {
            //     const LETTERS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
            //     let f = StringFaker::with(Vec::from(LETTERS), 8);
            //     f.fake()
            // };

            let dataset_id = format!("test_{}", rand::random::<u64>());

            // Create a new dataset
            let dataset = client
                .dataset()
                .create(Dataset::new(project_id, &dataset_id))
                .await
                .unwrap();

            create_table(&client, &dataset).await;

            Self {
                client,
                project_id: project_id.to_string(),
                dataset_id: dataset_id.to_string(),
                table_id: TABLE_ID.to_string(),
            }
        }

        pub async fn delete_dataset(&self) {
            // Delete the table previously created
            self.client
                .table()
                .delete(&self.project_id, &self.dataset_id, &self.table_id)
                .await
                .unwrap();

            // Delete the dataset previously created
            self.client
                .dataset()
                .delete(&self.project_id, &self.dataset_id, true)
                .await
                .unwrap();
        }

        pub async fn insert_row(&self, name: String) {
            let mut insert_request = TableDataInsertAllRequest::new();
            insert_request.add_row(None, Row { name }).unwrap();

            self.client
                .tabledata()
                .insert_all(
                    &self.project_id,
                    &self.dataset_id,
                    &self.table_id,
                    insert_request,
                )
                .await
                .unwrap();
        }

        pub async fn get_rows(&self) -> Vec<String> {
            let mut rs = self
                .client
                .job()
                .query(
                    &self.project_id,
                    QueryRequest::new(format!(
                        "SELECT * FROM `{}.{}.{}`",
                        &self.project_id, &self.dataset_id, &self.table_id
                    )),
                )
                .await
                .unwrap();

            let mut rows: Vec<String> = vec![];
            while rs.next_row() {
                let name = rs.get_string_by_name(NAME_COLUMN).unwrap().unwrap();
                rows.push(name)
            }
            rows
        }
    }

    async fn create_table(client: &Client, dataset: &Dataset) {
        dataset
            .create_table(
                client,
                Table::from_dataset(
                    dataset,
                    TABLE_ID,
                    TableSchema::new(vec![TableFieldSchema::string(NAME_COLUMN)]),
                ),
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn test_big_query_stub() {
    let docker = Cli::default();
    let project_id = format!("test_project_{}", rand::random::<u64>());
    let (gcp_client, _node, _temp_file, _auth) =
        client::new_bigquery_client(&docker, &project_id).await;

    let bq = BQ::new(gcp_client, &project_id).await;
    let name = "foo";
    bq.insert_row(name.to_string()).await;
    let rows = bq.get_rows().await;
    assert_eq!(rows, vec![name]);
    println!("That's all Folks!");
    bq.delete_dataset().await;
}

#[tokio::test]
async fn test_batch_insertion_of_blocks_and_receipts_retrieval_in_bq() {
    let docker = Cli::default();
    let project_id = format!("test_project_{}", rand::random::<u64>());
    let (gcp_client, _node, _temp_file, _auth) =
        client::new_bigquery_client(&docker, &project_id).await;
    let dataset_id = format!("test_{}", rand::random::<u64>());

    let mut blockchain = Box::new(
        BigQueryBlockChain::new_with_client(
            project_id.clone(),
            dataset_id.clone(),
            gcp_client.clone(),
        )
        .unwrap(),
    );

    blockchain.init().await.unwrap();

    let mut blocks = Vec::new();

    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        blocks.push(dummy_block);
    }

    let mut receipts = Vec::new();

    for _ in 1..=10 {
        let tx_hash = H256::random();
        let dummy_receipt: TransactionReceipt = ethers_core::types::TransactionReceipt {
            transaction_hash: tx_hash,
            ..Default::default()
        };

        receipts.push(dummy_receipt);
    }

    blockchain
        .insert_blocks_and_receipts(&blocks, &receipts)
        .await
        .unwrap();

    let block = blockchain.get_block_by_number(1).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 1);

    let receipt = blockchain
        .get_transaction_receipt(receipts[0].transaction_hash)
        .await
        .unwrap();

    assert_eq!(receipt.transaction_hash, receipts[0].transaction_hash);

    let block = blockchain.get_block_by_number(10).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 10);

    let receipt = blockchain
        .get_transaction_receipt(receipts[9].transaction_hash)
        .await
        .unwrap();

    assert_eq!(receipt.transaction_hash, receipts[9].transaction_hash);
}

#[tokio::test]
async fn test_getting_block_range() {
    let docker = Cli::default();
    let project_id = format!("test_project_{}", rand::random::<u64>());
    let (gcp_client, _node, _temp_file, _auth) =
        client::new_bigquery_client(&docker, &project_id).await;
    let dataset_id = format!("test_{}", rand::random::<u64>());

    let mut blockchain = Box::new(
        BigQueryBlockChain::new_with_client(
            project_id.clone(),
            dataset_id.clone(),
            gcp_client.clone(),
        )
        .unwrap(),
    );
    blockchain.init().await.unwrap();
    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        blockchain
            .insert_blocks_and_receipts(&[dummy_block], &[])
            .await
            .unwrap();
    }

    let block_range = blockchain.get_blocks_in_range(1, 10).await.unwrap();

    assert_eq!(block_range, (1..=10).collect::<Vec<u64>>());

    let block_range = blockchain.get_blocks_in_range(1, 5).await.unwrap();

    assert_eq!(block_range, (1..=5).collect::<Vec<u64>>());
}

#[tokio::test]
async fn test_retrieval_of_latest_and_oldest_block_number() {
    let docker = Cli::default();
    let project_id = format!("test_project_{}", rand::random::<u64>());
    let (gcp_client, _node, _temp_file, _auth) =
        client::new_bigquery_client(&docker, &project_id).await;
    let dataset_id = format!("test_{}", rand::random::<u64>());

    let mut blockchain = Box::new(
        BigQueryBlockChain::new_with_client(
            project_id.clone(),
            dataset_id.clone(),
            gcp_client.clone(),
        )
        .unwrap(),
    );

    blockchain.init().await.unwrap();

    for i in 1..=10 {
        let dummy_block: Block<Transaction> = ethers_core::types::Block {
            number: Some(ethers_core::types::U64::from(i)),
            hash: Some(H256::random()),
            ..Default::default()
        };

        blockchain
            .insert_blocks_and_receipts(&[dummy_block], &[])
            .await
            .unwrap();
    }

    let latest_block_number = blockchain.get_latest_block_number().await.unwrap();

    assert_eq!(latest_block_number, 10);

    let earliest_block_number = blockchain.get_earliest_block_number().await.unwrap();

    assert_eq!(earliest_block_number, 1);
}

#[tokio::test]
async fn test_init_idempotency() {
    let docker = Cli::default();
    let project_id = format!("test_project_{}", rand::random::<u64>());
    let (gcp_client, _node, _temp_file, _auth) =
        client::new_bigquery_client(&docker, &project_id).await;
    let dataset_id = format!("test_{}", rand::random::<u64>());

    let mut blockchain = Box::new(
        BigQueryBlockChain::new_with_client(
            project_id.clone(),
            dataset_id.clone(),
            gcp_client.clone(),
        )
        .unwrap(),
    );

    // Add a block
    let dummy_block: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(1)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    assert!(blockchain
        .insert_blocks_and_receipts(&[dummy_block], &[])
        .await
        .is_err());

    // First initialization - creates tables
    blockchain.init().await.unwrap();

    // Add a block
    let dummy_block: Block<Transaction> = ethers_core::types::Block {
        number: Some(ethers_core::types::U64::from(1)),
        hash: Some(H256::random()),
        ..Default::default()
    };

    assert!(blockchain
        .insert_blocks_and_receipts(&[dummy_block], &[])
        .await
        .is_ok());

    assert!(blockchain.init().await.is_ok());

    // Retrieve the block
    let block = blockchain.get_block_by_number(1).await.unwrap();

    assert_eq!(block.number.unwrap().as_u64(), 1);
}
