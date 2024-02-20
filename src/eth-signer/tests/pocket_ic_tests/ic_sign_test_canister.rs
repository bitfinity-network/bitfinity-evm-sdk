use candid::{Encode, Principal};
use ic_exports::pocket_ic::{self, PocketIc};

use crate::pocket_ic_tests::wasm_utils::get_test_canister_bytecode;

#[test]
fn test_canister_sign_and_check() {
    let env = pocket_ic::init_pocket_ic();
    let canister = deploy_canister(&env);

    let result = env.update_call(
        canister,
        Principal::anonymous(),
        "sign_and_check",
        Encode!(&()).unwrap(),
    );
    assert!(result.is_ok());
}

fn deploy_canister(env: &PocketIc) -> Principal {
    let dummy_wasm = get_test_canister_bytecode();
    let args = Encode!(&()).unwrap();
    let canister = env.create_canister();
    env.add_cycles(canister, 10_u128.pow(12));
    env.install_canister(canister, dummy_wasm.to_vec(), args, None);
    canister
}
