use ethers::prelude::*;

#[tokio::main]
async fn main() {
    let wallet = LocalWallet::new(&mut rand::thread_rng());
    println!("private key: {:?}", wallet.signer().to_bytes().to_vec());

    // ======== generate evm registry tx ========
    let tx = TransactionRequest::new()
        .from(wallet.address())
        .to("0xb0e5863d0ddf7e105e409fee0ecc0123a362e14b") // MINTER_ADDRESS
        .value(100_000) // REGISTRATION_FEE
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
