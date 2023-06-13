use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ethers::prelude::*;
use ic_agent::agent::http_transport::ReqwestHttpReplicaV2Transport;
use ic_agent::identity::AnonymousIdentity;
use ic_agent::{Agent, AgentError};

#[derive(Debug, Clone, CandidType, Deserialize)]
struct RegistrationInfo {
    minter_address: String,
    registration_fee: u64,
}

#[tokio::main]
async fn main() {
    let registration_info = get_minter_address().await.expect("call evmc error");

    let wallet = LocalWallet::new(&mut rand::thread_rng());
    println!("private key: {:?}", wallet.signer().to_bytes().to_vec());

    // ======== generate evm registry tx ========
    let tx = TransactionRequest::new()
        .from(wallet.address())
        .to(registration_info.minter_address.as_str()) // MINTER_ADDRESS
        .value(registration_info.registration_fee) // REGISTRATION_FEE
        .chain_id(355113) // evmc testnet chain id
        .gas(21000) // gas limit
        .gas_price(10) // use min gas price
        .nonce(0) // the new address's nonce is 0
        .into();

    let signature = wallet.sign_transaction(&tx).await.unwrap();

    println!(
        "r: {:#x}, s: {:#x}, v: {:#x}",
        signature.r, signature.s, signature.v
    );
    println!("tx hash: {:#x}", tx.hash(&signature));

    println!("tx: {:?}", tx);
}

async fn get_minter_address() -> Result<RegistrationInfo, AgentError> {
    let identity = AnonymousIdentity {};
    let transport = ReqwestHttpReplicaV2Transport::create("https://ic0.app")?;

    let agent = Agent::builder()
        .with_transport(transport)
        .with_identity(identity)
        .build()?;
    let evm_canister_id =
        Principal::from_text("4fe7g-7iaaa-aaaak-aegcq-cai").expect("error principal");

    let res = agent
        .query(&evm_canister_id, "registration_ic_agent_info")
        .with_arg(&Encode!(&()).expect("error encode none argument"))
        .call()
        .await?;

    let res = Decode!(res.as_slice(), RegistrationInfo).expect("decode response error");
    Ok(res)
}
