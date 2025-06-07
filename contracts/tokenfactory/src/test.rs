#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::{
    Env, String, Address, BytesN, testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
};

// Import the token contract WASM for upload (adjust path as needed)
// NOTE: The path must point to a built token.wasm file. Build the token contract first.
mod contract {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/tokenlaunch.wasm");
}

use crate::TokenFactory;
use crate::TokenFactoryClient;

#[test]
fn test_token_factory() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let salt = BytesN::from_array(&env, &[0; 32]);

    // Deploy TokenFactory
    let contract_id = env.register(TokenFactory, (&admin,));
    let factory = TokenFactoryClient::new(&env, &contract_id);

    // Upload token contract WASM and set hash
    let wasm_hash = env.deployer().upload_contract_wasm(contract::WASM);
    factory.update_pool_wasm_hash(&admin, &wasm_hash);

    // Create a new token via the factory
    let token_name = String::from_str(&env, "TestToken");
    let token_symbol = String::from_str(&env, "TTK");
    let token_decimals = 6u32;
    let token_supply = 1_000_000i128;
    let token_addr = factory.create_token(&token_name, &token_symbol, &token_decimals, &token_supply, &user, &salt);


    // Check that the token address is not the user address (sanity check)
    assert!(token_addr != user);
}
