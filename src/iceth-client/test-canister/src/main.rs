use iceth_client_test_canister::CounterCanister;

fn main() {
    let canister_e_idl = CounterCanister::idl();
    let idl =
        candid::bindings::candid::compile(&canister_e_idl.env.env, &Some(canister_e_idl.actor));

    println!("{}", idl);
}
