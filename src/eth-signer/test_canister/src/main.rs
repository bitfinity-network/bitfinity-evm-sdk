use crate::canister::TestCanister;

pub mod canister;

fn main() {
    let canister_e_idl = TestCanister::idl();
    let idl =
        candid::pretty::candid::compile(&canister_e_idl.env.env, &Some(canister_e_idl.actor));

    println!("{}", idl);
}
