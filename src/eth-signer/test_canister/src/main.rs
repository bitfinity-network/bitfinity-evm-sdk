use crate::canister::TestCanister;

pub mod canister;

fn main() {
    let canister_e_idl = TestCanister::idl();
    let idl =
        candid::bindings::candid::compile(&canister_e_idl.env.env, &Some(canister_e_idl.actor));

    println!("{}", idl);
}
