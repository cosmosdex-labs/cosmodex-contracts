#![cfg(test)]
extern crate alloc;
extern crate std;


use super::*;
use soroban_sdk::{
    // token::{self, TokenClient},
    vec, Env, String, Address, Symbol, FromVal, IntoVal,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    BytesN,
};

use crate::PoolFactory;
use crate::PoolFactoryClient;

use pool::LiquidityPool;
use pool::LiquidityPoolClient;

use ::token::Token;
use ::token::TokenClient;

mod contract {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/pool.wasm");
}


// Helper function to create a test token
fn create_token<'a>(e: &Env, admin: &Address) -> TokenClient<'a> {
    let token_contract = e.register(
        Token,
        (
            admin,
            18_u32,
            String::from_val(e, &"name"),
            String::from_val(e, &"symbol"),
        ),
    );
    TokenClient::new(e, &token_contract)
}

fn deploy_pool<'a>(e: &Env, token_a: &TokenClient<'a>, token_b: &TokenClient<'a>) -> LiquidityPoolClient<'a> {
    let contract_id = e.register(
        LiquidityPool,
        (
            &token_a.address,
            &token_b.address,
            String::from_val(e, &"LPToken"),
            String::from_val(e, &"LP"),
        ),
    );
    LiquidityPoolClient::new(e, &contract_id)
}

fn deploy_poolfactory<'a>(e: &Env, admin: &Address) -> PoolFactoryClient<'a> {
    let contract_id = e.register(PoolFactory, (admin,));
    PoolFactoryClient::new(e, &contract_id)
}



#[test]
fn test_pool_factory() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);

    let poolfactory = deploy_poolfactory(&env, &user);

    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    let salt = BytesN::from_array(&env, &[0; 32]);
    let wasm_hash = env.deployer().upload_contract_wasm(contract::WASM);
    poolfactory.update_pool_wasm_hash(&user, &wasm_hash);

    poolfactory.create_pool(
        &token_a.address,
        &token_b.address,
        &String::from_val(&env, &"LPToken"),
        &String::from_val(&env, &"LP"),
        &salt,
    );
    let pool_addr = poolfactory.get_pool(&token_a.address, &token_b.address);
    assert!(pool_addr.is_some());

      // Invoke contract to check that it is initialized.
      let client = LiquidityPoolClient::new(&env, &pool_addr.unwrap());
      assert_eq!(client.get_token_a(), token_a.address);
      assert_eq!(client.get_token_b(), token_b.address);
}
