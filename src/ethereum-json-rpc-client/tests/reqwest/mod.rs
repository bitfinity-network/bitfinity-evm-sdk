use std::sync::Arc;

use ethereum_json_rpc_client::{EthJsonRcpClient, reqwest::ReqwestClient};
use ethers_core::types::{BlockNumber, H256};

const ETHEREUM_JSON_API_URL: &str = "https://cloudflare-eth.com/";
const MAX_BATCH_SIZE: usize = 5;

fn to_hash(string: &str) -> H256 {
    H256::from_slice(
        hex::decode(string.trim_start_matches("0x"))
            .unwrap()
            .as_slice(),
    )
}

fn reqwest_client() -> EthJsonRcpClient<ReqwestClient> {
    EthJsonRcpClient::new(ReqwestClient::new(ETHEREUM_JSON_API_URL.to_string()))
}

#[tokio::test]
async fn should_get_block_number() {
    let result = reqwest_client().get_block_number().await.unwrap();
    assert!(result > 16896634);
}

#[tokio::test]
async fn should_get_block_by_number() {
    let result = reqwest_client().get_block_by_number(BlockNumber::Number(11588465.into()))
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
    let result =
    reqwest_client().
        get_full_block_by_number(BlockNumber::Number(11588465.into()))
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
    let result = reqwest_client().get_full_blocks_by_number(
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
async fn should_get_transaction_receipts() {
    let block = reqwest_client().get_block_by_number(BlockNumber::Number(11588465.into()))
        .await
        .unwrap();

    let receipts = reqwest_client().get_receipts_by_hash(
        vec![block.transactions[0], block.transactions[1]],
        MAX_BATCH_SIZE,
    )
    .await
    .unwrap();
    assert_eq!(receipts[0].gas_used, Some(21000.into()));
    assert_eq!(receipts[1].gas_used, Some(52358.into()));
}
