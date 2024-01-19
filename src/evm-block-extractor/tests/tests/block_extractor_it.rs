use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRcpClient;
use evm_block_extractor::block_extractor::BlockExtractor;

use crate::test_with_clients;

#[tokio::test]
async fn test_extractor_collect_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None).await.unwrap();

        let rpc_url = "https://testnet.bitfinity.network".to_string();
        let evm_client = Arc::new(EthJsonRcpClient::new(ReqwestClient::new(rpc_url)));

        let request_time_out_secs = 10;
        let rpc_batch_size = 10;
        let mut extractor = BlockExtractor::new(
            evm_client.clone(),
            request_time_out_secs,
            rpc_batch_size,
            db_client.clone(),
        );

        let end_block = evm_client.get_block_number().await.unwrap();
        let start_block = end_block - 10;

        println!("Getting blocks from {:?} to {}", start_block, end_block);

        let result = extractor
            .collect_blocks(start_block, end_block)
            .await
            .unwrap();

        assert_eq!(result.0, start_block);
        assert_eq!(result.1, end_block);

        for block_num in start_block..=end_block {
            let block = db_client.get_block_by_number(block_num).await.unwrap();

            let full_block = db_client.get_full_block_by_number(block_num).await.unwrap();

            // Check blocks
            {
                assert_eq!(block_num, full_block.number.unwrap().as_u64());
                assert_eq!(block_num, block.number.unwrap().as_u64());
                assert_eq!(block.hash.unwrap(), full_block.hash.unwrap());
            }

            // Check transactions
            {
                assert_eq!(block.transactions.len(), full_block.transactions.len());

                for tx in &full_block.transactions {
                    assert!(block.transactions.contains(&tx.hash));
                    assert_eq!(tx.block_number, tx.block_number);
                    assert_eq!(tx.block_hash, tx.block_hash);
                }
            }

            // Check receipts
            {
                for tx in &full_block.transactions {
                    let receipt = db_client.get_transaction_receipt(tx.hash).await.unwrap();

                    assert_eq!(tx.hash, receipt.transaction_hash);
                    assert_eq!(tx.block_number, receipt.block_number);
                    assert_eq!(tx.block_hash, receipt.block_hash);
                }
            }
        }
    })
    .await;
}
