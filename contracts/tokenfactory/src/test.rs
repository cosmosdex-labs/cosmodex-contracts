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
    let token_metadata = String::from_str(&env, "TTKLKDJFDFJKJKJFDFDF");

    let token_addr = factory.create_token(&admin, &token_name, &token_symbol, &token_decimals, &token_supply, &user, &token_metadata, &salt);


    // Check that the token address is not the user address (sanity check)
    assert!(token_addr != user);
    // Verify token is tracked in deployed tokens
    let deployed_tokens = factory.get_all_deployed_tokens();
    assert_eq!(deployed_tokens.len(), 1);
    assert_eq!(deployed_tokens.get(0).unwrap(), token_addr);
    
    // Verify individual token record exists
    env.as_contract(&contract_id, || {
        assert!(env.storage().instance().has(&DataKey::DeployedTokens(
            token_addr.clone(), 
            contract_id.clone()
        )));
    });
    
}

#[test]
fn test_get_all_deployed_tokens() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let salt1 = BytesN::from_array(&env, &[1; 32]);
    let salt2 = BytesN::from_array(&env, &[2; 32]);

    // Deploy TokenFactory
    let contract_id = env.register(TokenFactory, (&admin,));
    let factory = TokenFactoryClient::new(&env, &contract_id);

    // Upload token contract WASM and set hash
    let wasm_hash = env.deployer().upload_contract_wasm(contract::WASM);
    factory.update_pool_wasm_hash(&admin, &wasm_hash);

    // Verify initial state is empty
    let deployed_tokens = factory.get_all_deployed_tokens();
    assert_eq!(deployed_tokens.len(), 0);

    // Create first token
    let token_name1 = String::from_str(&env, "Token1");
    let token_symbol1 = String::from_str(&env, "TKN1");
    let token_metadata1 = String::from_str(&env, "TTKLKDJFDFJKJKJFDFDF");
    let token_addr1 = factory.create_token(
        &admin,
        &token_name1,
        &token_symbol1,
        &6u32,
        &1000i128,
        &user1,
        &token_metadata1,
        &salt1,
    );

    // Verify storage entry was created
    env.as_contract(&contract_id, || {
        assert!(env.storage().instance().has(&DataKey::DeployedTokens(
            token_addr1.clone(),
            contract_id.clone()
        )));
    });

    // Verify single token in list
    let deployed_tokens = factory.get_all_deployed_tokens();
    assert_eq!(deployed_tokens.len(), 1);
    assert_eq!(deployed_tokens.get(0).unwrap(), token_addr1);

    // Create second token
    let token_name2 = String::from_str(&env, "Token2");
    let token_symbol2 = String::from_str(&env, "TKN2");
    let token_addr2 = factory.create_token(
        &admin,
        &token_name2,
        &token_symbol2,
        &8u32,
        &2000i128,
        &user2,
        &token_metadata1,
        &salt2,
    );

    // Verify storage entry was created
    env.as_contract(&contract_id, || {
        assert!(env.storage().instance().has(&DataKey::DeployedTokens(
            token_addr2.clone(),
            contract_id.clone()
        )));
    });

    // Verify both tokens in correct order
    let deployed_tokens = factory.get_all_deployed_tokens();
    assert_eq!(deployed_tokens.len(), 2);
    assert_eq!(deployed_tokens.get(0).unwrap(), token_addr1);
    assert_eq!(deployed_tokens.get(1).unwrap(), token_addr2);
}