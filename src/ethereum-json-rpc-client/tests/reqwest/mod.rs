mod rpc_client;

use std::time::Duration;

use alloy::dyn_abi::{DynSolValue, FunctionExt, JsonAbiExt};
use alloy::json_abi::Function;
use alloy::rpc::types::{TransactionInput, TransactionRequest};
use did::{BlockNumber, H160, H256, U256};
use ethereum_json_rpc_client::{EthGetLogsParams, EthJsonRpcClient};
use rpc_client::RpcReqwestClient;
use serial_test::serial;

const MAX_BATCH_SIZE: usize = 5;

fn to_hash(string: &str) -> H256 {
    H256::from_hex_str(string).unwrap()
}

fn reqwest_client() -> EthJsonRpcClient<RpcReqwestClient> {
    let rpc_client = match std::env::var("ALCHEMY_API_KEY").ok() {
        Some(apikey) => {
            log::info!("ALCHEMY_API_KEY set, using Alchemy RPC endpoint");
            RpcReqwestClient::alchemy(apikey)
        }
        None => {
            log::warn!("ALCHEMY_API_KEY not set, using public RPC endpoint");
            RpcReqwestClient::public()
        }
    };

    EthJsonRpcClient::new(rpc_client)
}

#[tokio::test]
#[serial]
async fn should_get_block_number() {
    let result = reqwest_client().get_block_number().await.unwrap();
    assert!(result > 16896634);
}

#[tokio::test]
#[serial]
async fn should_get_balance() {
    let erc_1820_deployer_address =
        H160::from_hex_str("0xa990077c3205cbDf861e17Fa532eeB069cE9fF96").unwrap();
    let result = reqwest_client()
        .get_balance(erc_1820_deployer_address, BlockNumber::Latest)
        .await
        .unwrap();
    assert_eq!(result, 1409174700000000000u64.into());
}

#[tokio::test]
#[serial]
async fn should_get_gas_price() {
    let price = reqwest_client().gas_price().await.unwrap();
    assert!(price > U256::zero());
}

#[tokio::test]
#[serial]
async fn should_get_code() {
    let erc_1820_address =
        H160::from_hex_str("0x1820a4B7618BdE71Dce8cdc73aAB6C95905faD24").unwrap();
    let result = reqwest_client()
        .get_code(erc_1820_address, BlockNumber::Latest)
        .await
        .unwrap();
    assert_eq!(result, ERC_1820_EXPECTED_CODE);
}

/// Calls the funtction of ERC-1820:
///
///```solidity
///     function getManager(address _addr) public view returns(address)
///```
#[tokio::test]
#[serial]
async fn should_perform_eth_call() {
    let erc_1820_address =
        H160::from_hex_str("0x1820a4B7618BdE71Dce8cdc73aAB6C95905faD24").unwrap();

    let caller = H160::from_hex_str("0xf990077c3205cbDf861e17Fa532eeB069cE9fF96").unwrap();

    let func =
        Function::parse("function getManager(address _addr) public view returns(address)").unwrap();
    let input = func
        .abi_encode_input(&[DynSolValue::Address(caller.0)])
        .unwrap();

    let params = TransactionRequest {
        from: Some(caller.0),
        to: Some(erc_1820_address.0.into()),
        gas: Some(1000000u64),
        gas_price: None,
        value: None,
        input: TransactionInput::from(input),
        ..Default::default()
    };

    let result = reqwest_client()
        .eth_call(params, BlockNumber::Latest)
        .await
        .unwrap();

    let result_address = func
        .abi_decode_output(
            &alloy::hex::decode(result.trim_start_matches("0x")).unwrap(),
            false,
        )
        .unwrap()
        .first()
        .cloned()
        .unwrap()
        .as_address()
        .unwrap();

    assert_eq!(result_address, caller.0);
}

#[tokio::test]
#[serial]
async fn should_get_transaction_count() {
    let erc_1820_deployer_address =
        H160::from_hex_str("0xa990077c3205cbDf861e17Fa532eeB069cE9fF96").unwrap();
    let result = reqwest_client()
        .get_transaction_count(erc_1820_deployer_address, BlockNumber::Latest)
        .await
        .unwrap();
    assert_eq!(result, 1u64);
}

#[tokio::test]
#[serial]
async fn should_get_block_by_number() {
    let result = reqwest_client()
        .get_block_by_number(BlockNumber::Number(11588465.into()))
        .await
        .unwrap();

    let expected_hash =
        to_hash("0x719c3309fe7052a7adf34954418e1458c48d0e4b899d1d833d291ae6369f3500");
    let expected_state_root =
        to_hash("0xc9df81d6e32ac7b110c73ac283cfc84b97714a8e5fcaf36f1ff04822494e83fd");

    assert_eq!(result.hash, expected_hash);
    assert_eq!(result.state_root, expected_state_root);
    assert_eq!(result.transactions.len(), 265);
}

#[tokio::test]
#[serial]
async fn should_get_full_block_by_number() {
    let result = reqwest_client()
        .get_full_block_by_number(BlockNumber::Number(11588465.into()))
        .await
        .unwrap();

    let expected_hash =
        to_hash("0x719c3309fe7052a7adf34954418e1458c48d0e4b899d1d833d291ae6369f3500");
    let expected_state_root =
        to_hash("0xc9df81d6e32ac7b110c73ac283cfc84b97714a8e5fcaf36f1ff04822494e83fd");

    assert_eq!(result.hash, expected_hash);
    assert_eq!(result.state_root, expected_state_root);
    assert_eq!(result.transactions.len(), 265);

    assert_eq!(
        result.transactions[0].hash,
        to_hash("0x3adf87cb6ed6cf384317a28028295816fd971e17368c2a346a95fa654e80edc4")
    );
}

#[tokio::test]
#[serial]
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
        to_hash("0x719c3309fe7052a7adf34954418e1458c48d0e4b899d1d833d291ae6369f3500",)
    );
    assert_eq!(
        result[0].state_root,
        to_hash("0xc9df81d6e32ac7b110c73ac283cfc84b97714a8e5fcaf36f1ff04822494e83fd",)
    );
    assert_eq!(result[0].transactions.len(), 265);

    assert_eq!(
        result[1].hash,
        to_hash("0x78bc6c4e6a8628f4ffea4cc4d9413ed8a902a28ef7e4dd6332ead280abd77e61",)
    );
    assert_eq!(
        result[1].state_root,
        to_hash("0x272cd4af7a077a7cf1f41fdb03810f04628ea8ba6c60222ddea89333c0e59b9b",)
    );
    assert_eq!(result[1].transactions.len(), 222);
}

#[tokio::test]
#[serial]
async fn should_get_logs() {
    let params = EthGetLogsParams {
        address: Some(vec![H160::from_hex_str(
            "0xb59f67a8bff5d8cd03f6ac17265c550ed8f33907",
        )
        .unwrap()]),
        from_block: BlockNumber::Latest,
        to_block: BlockNumber::Latest,
        topics: Some(vec![
            vec![H256::from_hex_str(
                "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
            )
            .unwrap()],
            vec![H256::from_hex_str(
                "0x00000000000000000000000000b46c2526e227482e2ebb8f4c69e4674d262e75",
            )
            .unwrap()],
            vec![H256::from_hex_str(
                "0x00000000000000000000000054a2d42a40f51259dedd1978f6c118a0f0eff078",
            )
            .unwrap()],
        ]),
    };

    assert!(reqwest_client().get_logs(params).await.is_ok());
}

#[tokio::test]
#[serial]
async fn should_get_transaction_receipts() {
    // this test is flaky for some reasons, so we try multiple times
    for _ in 0..3 {
        let block = reqwest_client()
            .get_block_by_number(BlockNumber::Number(11588465.into()))
            .await
            .unwrap();
        if let Ok(receipts) = reqwest_client()
            .get_receipts_by_hash(
                vec![block.transactions[0].clone(), block.transactions[1].clone()],
                MAX_BATCH_SIZE,
            )
            .await
        {
            assert_eq!(receipts[0].gas_used, Some(21000u64.into()));
            assert_eq!(receipts[1].gas_used, Some(52358u64.into()));
            return;
        } else {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    panic!("Failed to get transaction receipts");
}

const ERC_1820_EXPECTED_CODE: &str = "0x608060405234801561001057600080fd5b50600436106100a557600035\
7c010000000000000000000000000000000000000000000000000000000090048063a41e7d5111610078578063a41e7d51\
146101d4578063aabbb8ca1461020a578063b705676514610236578063f712f3e814610280576100a5565b806329965a1d\
146100aa5780633d584063146100e25780635df8122f1461012457806365ba36c114610152575b600080fd5b6100e06004\
80360360608110156100c057600080fd5b50600160a060020a038135811691602081013591604090910135166102b6565b\
005b610108600480360360208110156100f857600080fd5b5035600160a060020a0316610570565b60408051600160a060\
020a039092168252519081900360200190f35b6100e06004803603604081101561013a57600080fd5b50600160a060020a\
03813581169160200135166105bc565b6101c26004803603602081101561016857600080fd5b8101906020810181356401\
0000000081111561018357600080fd5b82018360208201111561019557600080fd5b803590602001918460018302840111\
640100000000831117156101b757600080fd5b5090925090506106b3565b60408051918252519081900360200190f35b61\
00e0600480360360408110156101ea57600080fd5b508035600160a060020a03169060200135600160e060020a03191661\
06ee565b6101086004803603604081101561022057600080fd5b50600160a060020a038135169060200135610778565b61\
026c6004803603604081101561024c57600080fd5b508035600160a060020a03169060200135600160e060020a03191661\
07ef565b604080519115158252519081900360200190f35b61026c6004803603604081101561029657600080fd5b508035\
600160a060020a03169060200135600160e060020a0319166108aa565b6000600160a060020a038416156102cd57836102\
cf565b335b9050336102db82610570565b600160a060020a031614610339576040805160e560020a62461bcd0281526020\
6004820152600f60248201527f4e6f7420746865206d616e61676572000000000000000000000000000000000060448201\
5290519081900360640190fd5b6103428361092a565b15610397576040805160e560020a62461bcd028152602060048201\
52601a60248201527f4d757374206e6f7420626520616e2045524331363520686173680000000000006044820152905190\
81900360640190fd5b600160a060020a038216158015906103b85750600160a060020a0382163314155b156104ff576040\
5160200180807f455243313832305f4143434550545f4d4147494300000000000000000000000081525060140190506040\
516020818303038152906040528051906020012082600160a060020a031663249cb3fa85846040518363ffffffff167c01\
000000000000000000000000000000000000000000000000000000000281526004018083815260200182600160a060020a\
0316600160a060020a031681526020019250505060206040518083038186803b15801561047e57600080fd5b505afa1580\
15610492573d6000803e3d6000fd5b505050506040513d60208110156104a857600080fd5b5051146104ff576040805160\
e560020a62461bcd02815260206004820181905260248201527f446f6573206e6f7420696d706c656d656e742074686520\
696e74657266616365604482015290519081900360640190fd5b600160a060020a03818116600081815260208181526040\
808320888452909152808220805473ffffffffffffffffffffffffffffffffffffffff1916948716948517905551869291\
7f93baa6efbd2244243bfee6ce4cfdd1d04fc4c0e9a786abd3a41313bd352db15391a450505050565b600160a060020a03\
818116600090815260016020526040812054909116151561059a5750806105b7565b50600160a060020a03808216600090\
815260016020526040902054165b919050565b336105c683610570565b600160a060020a031614610624576040805160e5\
60020a62461bcd02815260206004820152600f60248201527f4e6f7420746865206d616e61676572000000000000000000\
0000000000000000604482015290519081900360640190fd5b81600160a060020a031681600160a060020a031614610643\
5780610646565b60005b600160a060020a03838116600081815260016020526040808220805473ffffffffffffffffffff\
ffffffffffffffffffff19169585169590951790945592519184169290917f605c2dbf762e5f7d60a546d42e7205dcb1b0\
11ebc62a61736a57c9089d3a43509190a35050565b60008282604051602001808383808284378083019250505092505050\
6040516020818303038152906040528051906020012090505b92915050565b6106f882826107ef565b6107035760006107\
05565b815b600160a060020a03928316600081815260208181526040808320600160e060020a0319969096168084529582\
52808320805473ffffffffffffffffffffffffffffffffffffffff19169590971694909417909555908152600284528181\
209281529190925220805460ff19166001179055565b600080600160a060020a038416156107905783610792565b335b90\
5061079d8361092a565b156107c357826107ad82826108aa565b6107b85760006107ba565b815b925050506106e8565b60\
0160a060020a0390811660009081526020818152604080832086845290915290205416905092915050565b600080806108\
1d857f01ffc9a70000000000000000000000000000000000000000000000000000000061094c565b909250905081158061\
082d575080155b1561083d576000925050506106e8565b61084f85600160e060020a031961094c565b9092509050811580\
61086057508015155b15610870576000925050506106e8565b61087a858561094c565b909250905060018214801561088f\
5750806001145b1561089f576001925050506106e8565b506000949350505050565b600160a060020a0382166000908152\
600260209081526040808320600160e060020a03198516845290915281205460ff1615156108f2576108eb83836107ef56\
5b90506106e8565b50600160a060020a03808316600081815260208181526040808320600160e060020a03198716845290\
91529020549091161492915050565b7bffffffffffffffffffffffffffffffffffffffffffffffffffffffff161590565b\
6040517f01ffc9a70000000000000000000000000000000000000000000000000000000080825260048201839052600091\
82919060208160248189617530fa90519096909550935050505056fea165627a7a72305820377f4a2d4301ede9949f163f\
319021a6e9c687c292a5e2b2c4734c126b524e6c0029";
