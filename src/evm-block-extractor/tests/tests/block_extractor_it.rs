use std::sync::Arc;

use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::EthJsonRpcClient;
use evm_block_extractor::database::AccountBalance;
use evm_block_extractor::task::block_extractor::BlockExtractor;

use crate::test_with_clients;

#[tokio::test]
async fn test_extractor_collect_blocks() {
    test_with_clients(|db_client| async move {
        db_client.init(None, false).await.unwrap();

        let rpc_url = "https://testnet.bitfinity.network".to_string();
        let evm_client = Arc::new(EthJsonRpcClient::new(ReqwestClient::new(rpc_url)));

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

        // Check genesis accounts
        {
            let evmc_genesis_balances = evm_client.get_genesis_balances().await.unwrap();
            let db_genesis_balances = db_client.get_genesis_balances().await.unwrap().unwrap();

            assert!(!evmc_genesis_balances.is_empty());

            let evmc_genesis_balances = evmc_genesis_balances
                .into_iter()
                .map(|(address, balance)| AccountBalance {
                    address: address.into(),
                    balance: balance.into(),
                })
                .collect::<Vec<_>>();

            assert_eq!(evmc_genesis_balances, db_genesis_balances);
        }

        // Check chain id
        {
            let evmc_chain_id = evm_client.get_chain_id().await.unwrap();
            let db_chain_id = db_client.get_chain_id().await.unwrap().unwrap();

            assert_eq!(evmc_chain_id, db_chain_id);
        }

        for block_num in start_block..=end_block {
            let block = db_client.get_block_by_number(block_num).await.unwrap();

            let full_block = db_client.get_full_block_by_number(block_num).await.unwrap();

            // Check blocks
            {
                assert_eq!(block_num, full_block.number.0.as_u64());
                assert_eq!(block_num, block.number.0.as_u64());
                assert_eq!(block.hash, full_block.hash);
            }

            // Check transactions
            {
                println!(
                    "Found transactions for block {}: {}",
                    block_num,
                    block.transactions.len()
                );
                assert_eq!(block.transactions.len(), full_block.transactions.len());

                for tx in &full_block.transactions {
                    assert!(block.transactions.contains(&tx.hash));
                    assert_eq!(tx.block_number, tx.block_number);
                    assert_eq!(tx.block_hash, tx.block_hash);
                }
            }
        }
    })
    .await;
}
