#![cfg(test)]
extern crate std;
use soroban_sdk::{
    // token::{self, TokenClient},
    vec, Env, String, Address, Symbol, FromVal, IntoVal,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
};
use crate::LiquidityPool;
use crate::LiquidityPoolClient;
use ::token::Token;
use ::token::TokenClient;

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

#[test]
fn test_add_initial_liquidity() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint some tokens to the user
    token_a.mint(&user, &10_000_000_000);
    token_b.mint(&user, &10_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    
    // Add initial liquidity
    let amount_a = 10_000_000_000;
    let amount_b = 10_000_000_000;
    
    // // Approve the pool to spend user's tokens
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    
    // Add liquidity
    let liquidity = pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Verify LP tokens minted (sqrt(amount_a * amount_b) = sqrt(10*10) = 10)
    assert_eq!(liquidity, 10_000_000_000);
    assert_eq!(pool.balance(&user), 10_000_000_000);
    
    // Verify reserves updated
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, amount_a);
    assert_eq!(reserve_b, amount_b);
}


#[test]
fn test_add_proportional_liquidity() {
    let env = Env::default();
    env.mock_all_auths();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user1);
    let token_b = create_token(&env, &user1);
    
    // Mint tokens to users
    token_a.mint(&user1, &30_000_000_000);
    token_b.mint(&user1, &30_000_000_000);
    token_a.mint(&user2, &20_000_000_000);
    token_b.mint(&user2, &20_000_000_000);
    
     // Deploy pool contract
     let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Approve and add initial liquidity from user1
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    token_a.approve(&user1, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user1, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user1, &initial_amount_a, &initial_amount_b);
    
    // Approve and add proportional liquidity from user2
    let prop_amount_a = 20_000_000_000;
    let prop_amount_b = 20_000_000_000;
    token_a.approve(&user2, &pool.address, &prop_amount_a, &1000);
    token_b.approve(&user2, &pool.address, &prop_amount_b, &1000);
    let liquidity = pool.add_liquidity(&user2, &prop_amount_a, &prop_amount_b);
    
    // Verify LP tokens minted to user2
    assert_eq!(liquidity, 20_000_000_000);
    assert_eq!(pool.balance(&user2), 20_000_000_000);
    
    // Verify total supply
    assert_eq!(pool.supply(), 30_000_000_000);
    
    // Verify reserves updated
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, initial_amount_a + prop_amount_a);
    assert_eq!(reserve_b, initial_amount_b + prop_amount_b);
}

#[test]
#[should_panic(expected = "Amounts must be proportional")]
fn test_add_non_proportional_liquidity_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &20_000_000_000);
    token_b.mint(&user, &25_000_000_000);
    
     // Deploy pool contract
     let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Approve and add initial liquidity
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    
    token_a.approve(&user, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user, &initial_amount_a, &initial_amount_b);
    
    // Attempt to add non-proportional liquidity
    let non_prop_amount_a = 10_000_000_000;
    let non_prop_amount_b = 15_000_000_000;
    token_a.approve(&user, &pool.address, &non_prop_amount_a, &1000);
    token_b.approve(&user, &pool.address, &non_prop_amount_b, &1000);
    pool.add_liquidity(&user, &non_prop_amount_a, &non_prop_amount_b);
}

#[test]
#[should_panic(expected = "Insufficient liquidity minted")]
fn test_add_liquidity_zero_amounts_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    
    // Attempt to add zero liquidity
    pool.add_liquidity(&user, &0, &0);
}

#[test]
fn test_remove_liquidity() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &10_000_000_000);
    token_b.mint(&user, &10_000_000_000);
    
   // Deploy pool contract
   let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Approve and add initial liquidity
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user, &initial_amount_a, &initial_amount_b);
    
    let initial_lp_supply = pool.supply();
    
    // Remove half of the liquidity
    let liquidity_to_remove = 5_000_000_000;
    let (amount_a_out, amount_b_out) = pool.remove_liquidity(&user, &liquidity_to_remove);
    
    // Verify LP tokens burned
    assert_eq!(pool.balance(&user), initial_lp_supply - liquidity_to_remove);
    assert_eq!(pool.supply(), initial_lp_supply - liquidity_to_remove);
    
    // Verify tokens returned
    assert_eq!(amount_a_out, 5_000_000_000);
    assert_eq!(amount_b_out, 5_000_000_000);
    
    // Verify reserves updated
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, initial_amount_a - amount_a_out);
    assert_eq!(reserve_b, initial_amount_b - amount_b_out);
}

#[test]
#[should_panic(expected = "No liquidity in pool")]
fn test_remove_liquidity_from_empty_pool_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Attempt to remove liquidity from empty pool
    pool.remove_liquidity(&user, &100);
}

#[test]
#[should_panic(expected = "Liquidity must be > 0")]
fn test_remove_zero_liquidity_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &10_000_000_000);
    token_b.mint(&user, &10_000_000_000);
    
     // Deploy pool contract
     let pool = deploy_pool(&env, &token_a, &token_b);
    // Approve and add some liquidity
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user, &initial_amount_a, &initial_amount_b);
    
    // Attempt to remove zero liquidity
    pool.remove_liquidity(&user, &0);
}

#[test]
#[should_panic(expected = "Insufficient LP tokens")]
fn test_remove_insufficient_lp_tokens_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user1);
    let token_b = create_token(&env, &user1);
    
    // Mint tokens
    token_a.mint(&user1, &10_000_000_000);
    token_b.mint(&user1, &10_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    // Approve and add initial liquidity from user1
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    token_a.approve(&user1, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user1, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user1, &initial_amount_a, &initial_amount_b);
    
    // Attempt to remove liquidity from user2 (who has no LP tokens)
    pool.remove_liquidity(&user2, &1);
}

#[test]
fn test_swap_a_for_b() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &20_000_000_000);
    token_b.mint(&user, &10_000_000_000);
    
    
     // Deploy pool contract
     let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Approve and add initial liquidity
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user, &initial_amount_a, &initial_amount_b);
    
    // Perform swap
    let amount_in = 10_000_000_000;
    token_a.approve(&user, &pool.address, &amount_in, &1000);
    let amount_out = pool.swap(&user, &token_a.address, &amount_in);
    
    // Calculate expected amount out (with 0.3% fee)
    let amount_in_with_fee = amount_in * (10000 - 30) / 10000;
    let expected_amount_out = (initial_amount_b * amount_in_with_fee) / (initial_amount_a + amount_in_with_fee);
    
    // Verify swap amount
    assert_eq!(amount_out, expected_amount_out);
    
    // Verify reserves updated
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, initial_amount_a + amount_in);
    assert_eq!(reserve_b, initial_amount_b - amount_out);
}

#[test]
#[should_panic(expected = "Amount in must be > 0")]
fn test_swap_zero_amount_in_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &10_000_000_000);
    token_b.mint(&user, &10_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Approve and add initial liquidity
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user, &initial_amount_a, &initial_amount_b);
    
    // Attempt to swap zero amount
    pool.swap(&user, &token_a.address, &0);
}

#[test]
#[should_panic(expected = "Invalid token address")]
fn test_swap_invalid_token_address_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    let invalid_token = Address::generate(&env);
    
    // Mint tokens
    token_a.mint(&user, &10_000_000_000);
    token_b.mint(&user, &10_000_000_000);
    
     // Deploy pool contract
     let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Approve and add initial liquidity
    let initial_amount_a = 10_000_000_000;
    let initial_amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user, &pool.address, &initial_amount_b, &1000);
    pool.add_liquidity(&user, &initial_amount_a, &initial_amount_b);
    
    // Attempt to swap using invalid token
    pool.swap(&user, &invalid_token, &1);
}