use ethereum_json_rpc_client::reqwest::ReqwestClient;
use ethereum_json_rpc_client::{EthGetLogsParams, EthJsonRpcClient};
use ethers_core::types::{BlockNumber, Log, H256};

const ETHEREUM_JSON_API_URL: &str = "https://cloudflare-eth.com/";
const MAX_BATCH_SIZE: usize = 5;

fn to_hash(string: &str) -> H256 {
    H256::from_slice(
        hex::decode(string.trim_start_matches("0x"))
            .unwrap()
            .as_slice(),
    )
}

fn reqwest_client() -> EthJsonRpcClient<ReqwestClient> {
    EthJsonRpcClient::new(ReqwestClient::new(ETHEREUM_JSON_API_URL.to_string()))
}

#[tokio::test]
async fn should_get_block_number() {
    let result = reqwest_client().get_block_number().await.unwrap();
    assert!(result > 16896634);
}

#[tokio::test]
async fn should_get_balance() {
    let erc_1820_address = "0xa990077c3205cbDf861e17Fa532eeB069cE9fF96"
        .parse()
        .unwrap();
    let result = reqwest_client()
        .get_balance(erc_1820_address, BlockNumber::Latest)
        .await
        .unwrap();
    assert_eq!(result, 1409174700000000000u64.into());
}

#[tokio::test]
async fn should_get_transaction_count() {
    let erc_1820_address = "0xa990077c3205cbDf861e17Fa532eeB069cE9fF96"
        .parse()
        .unwrap();
    let result = reqwest_client()
        .get_transaction_count(erc_1820_address, BlockNumber::Latest)
        .await
        .unwrap();
    assert_eq!(result, 1u64);
}

#[tokio::test]
async fn should_get_block_by_number() {
    let result = reqwest_client()
        .get_block_by_number(BlockNumber::Number(11588465.into()))
        .await
        .unwrap();

    let expected_hash =
        to_hash("0x719c3309fe7052a7adf34954418e1458c48d0e4b899d1d833d291ae6369f3500");
    let expected_state_root =
        to_hash("0xc9df81d6e32ac7b110c73ac283cfc84b97714a8e5fcaf36f1ff04822494e83fd");

    assert_eq!(result.hash, Some(expected_hash));
    assert_eq!(result.state_root, expected_state_root);
    assert_eq!(result.transactions.len(), 265);
}

#[tokio::test]
async fn should_get_full_block_by_number() {
    let result = reqwest_client()
        .get_full_block_by_number(BlockNumber::Number(11588465.into()))
        .await
        .unwrap();

    let expected_hash =
        to_hash("0x719c3309fe7052a7adf34954418e1458c48d0e4b899d1d833d291ae6369f3500");
    let expected_state_root =
        to_hash("0xc9df81d6e32ac7b110c73ac283cfc84b97714a8e5fcaf36f1ff04822494e83fd");

    assert_eq!(result.hash, Some(expected_hash));
    assert_eq!(result.state_root, expected_state_root);
    assert_eq!(result.transactions.len(), 265);

    assert_eq!(
        result.transactions[0].hash,
        to_hash("0x3adf87cb6ed6cf384317a28028295816fd971e17368c2a346a95fa654e80edc4")
    );
}

#[tokio::test]
async fn should_get_full_blocks_by_number() {
    let result = reqwest_client()
        .get_full_blocks_by_number(
            vec![
                BlockNumber::Number(11588465.into()),
                BlockNumber::Number(11588466.into()),
            ],
            MAX_BATCH_SIZE,
        )
        .await
        .unwrap();

    assert_eq!(result.len(), 2);

    assert_eq!(
        result[0].hash,
        Some(to_hash(
            "0x719c3309fe7052a7adf34954418e1458c48d0e4b899d1d833d291ae6369f3500",
        ))
    );
    assert_eq!(
        result[0].state_root,
        to_hash("0xc9df81d6e32ac7b110c73ac283cfc84b97714a8e5fcaf36f1ff04822494e83fd",)
    );
    assert_eq!(result[0].transactions.len(), 265);

    assert_eq!(
        result[1].hash,
        Some(to_hash(
            "0x78bc6c4e6a8628f4ffea4cc4d9413ed8a902a28ef7e4dd6332ead280abd77e61",
        ))
    );
    assert_eq!(
        result[1].state_root,
        to_hash("0x272cd4af7a077a7cf1f41fdb03810f04628ea8ba6c60222ddea89333c0e59b9b",)
    );
    assert_eq!(result[1].transactions.len(), 222);
}

#[tokio::test]
async fn should_get_logs() {
    let params = EthGetLogsParams {
        address: Some(vec!["0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907"
            .parse()
            .unwrap()]),
        from_block: "0x429d3b".parse().unwrap(),
        to_block: BlockNumber::Latest,
        topics: Some(vec![
            vec![
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
                    .parse()
                    .unwrap(),
            ],
            vec![
                "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75"
                    .parse()
                    .unwrap(),
            ],
            vec![
                "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
                    .parse()
                    .unwrap(),
            ],
        ]),
    };

    let result = reqwest_client().get_logs(params).await.unwrap();

    let expected_result: Vec<Log> = serde_json::from_str(
        r#"[
            {
                "address": "0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907",
                "blockHash": "0x8243343df08b9751f5ca0c5f8c9c0460d8a9b6351066fae0acbd4d3e776de8bb",
                "blockNumber": "0x429d3b",
                "data": "0x000000000000000000000000000000000000000000000000000000012a05f200",
                "logIndex": "0x56",
                "removed": false,
                "topics": [
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                    "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
                    "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078"
                ],
                "transactionHash": "0xab059a62e22e230fe0f56d8555340a29b2e9532360368f810595453f6fdd213b",
                "transactionIndex": "0xac"
            }
        ]"#
    ).unwrap();

    assert_eq!(result, expected_result);
}

#[tokio::test]
async fn should_get_transaction_receipts() {
    let block = reqwest_client()
        .get_block_by_number(BlockNumber::Number(11588465.into()))
        .await
        .unwrap();

    let receipts = reqwest_client()
        .get_receipts_by_hash(
            vec![block.transactions[0], block.transactions[1]],
            MAX_BATCH_SIZE,
        )
        .await
        .unwrap();
    assert_eq!(receipts[0].gas_used, Some(21000.into()));
    assert_eq!(receipts[1].gas_used, Some(52358.into()));
}
