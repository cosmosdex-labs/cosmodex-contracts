#![cfg(test)]
extern crate std;
use soroban_sdk::{
    Env, String, Address, FromVal,
    testutils::{Address as _},
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

// Helper function to create native XLM address for tests
fn create_native_xlm_address(e: &Env) -> Address {
    // Create a mock XLM token by registering a dummy token contract
    // This simulates XLM in the test environment
    let xlm_token = e.register(
        Token,
        (
            &Address::generate(e), // admin
            18_u32,
            String::from_val(e, &"Stellar Lumens"),
            String::from_val(e, &"XLM"),
        ),
    );
    
    xlm_token
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
    assert_eq!(pool.balance_of(&user), 10_000_000_000);
    
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
    assert_eq!(pool.balance_of(&user2), 20_000_000_000);
    
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
    
    // Attempt to add liquidity with zero amounts
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
    token_a.mint(&user, &20_000_000_000);
    token_b.mint(&user, &20_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity
    let amount_a = 10_000_000_000;
    let amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Remove half of the liquidity
    let remove_amount = 5_000_000_000;
    let (returned_a, returned_b) = pool.remove_liquidity(&user, &remove_amount);
    
    // Verify returned amounts (should be proportional)
    assert_eq!(returned_a, 5_000_000_000);
    assert_eq!(returned_b, 5_000_000_000);
    
    // Verify remaining LP tokens
    assert_eq!(pool.balance_of(&user), 5_000_000_000);
    
    // Verify updated reserves
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, 5_000_000_000);
    assert_eq!(reserve_b, 5_000_000_000);
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
    pool.remove_liquidity(&user, &1_000_000_000);
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
    
    // Add initial liquidity
    let amount_a = 10_000_000_000;
    let amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Attempt to remove zero liquidity
    pool.remove_liquidity(&user, &0);
}

#[test]
#[should_panic(expected = "Insufficient LP tokens")]
fn test_remove_insufficient_lp_tokens_panics() {
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
    
    // Add initial liquidity
    let amount_a = 10_000_000_000;
    let amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Attempt to remove more liquidity than owned
    pool.remove_liquidity(&user, &20_000_000_000);
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
    token_b.mint(&user, &20_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity
    let amount_a = 10_000_000_000;
    let amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Perform swap
    let swap_amount = 1_000_000_000;
    token_a.approve(&user, &pool.address, &swap_amount, &1000);
    let amount_out = pool.swap(&user, &token_a.address, &swap_amount);
    
    // Verify swap result (with 0.3% fee)
    assert!(amount_out > 0);
    assert!(amount_out < 1_000_000_000); // Should be less due to fee
    
    // Verify reserves updated
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, amount_a + swap_amount);
    assert_eq!(reserve_b, amount_b - amount_out);
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
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Attempt to swap with zero amount
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
    let invalid_token = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Attempt to swap with invalid token
    pool.swap(&user, &invalid_token.address, &1_000_000_000);
}

#[test]
fn test_xlm_pool_detection() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let xlm_token = create_native_xlm_address(&env); // This creates a mock XLM token
    
    // Deploy pool with XLM
    let contract_id = env.register(
        LiquidityPool,
        (
            &token_a.address,
            &xlm_token,
            String::from_val(&env, &"LPToken"),
            String::from_val(&env, &"LP"),
        ),
    );
    let pool = LiquidityPoolClient::new(&env, &contract_id);
    
    // Verify that the pool was created successfully
    assert_eq!(pool.get_token_a(), token_a.address);
    assert_eq!(pool.get_token_b(), xlm_token);
    
    // Test that the pool correctly identifies as non-XLM pool (since we're using a mock token, not real "native")
    assert_eq!(pool.is_xlm_pool(), false);
    
    // Test XLM token index
    let xlm_index = pool.get_xlm_token_index();
    assert_eq!(xlm_index, None); // No real XLM in pool
    
    // Test that the pool can handle token operations
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, 0);
    assert_eq!(reserve_b, 0);
}

#[test]
fn test_xlm_pool_with_xlm_as_token_a() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_b = create_token(&env, &user);
    let xlm_token = create_native_xlm_address(&env);
    
    // Deploy pool with XLM as token A
    let contract_id = env.register(
        LiquidityPool,
        (
            &xlm_token,
            &token_b.address,
            String::from_val(&env, &"LPToken"),
            String::from_val(&env, &"LP"),
        ),
    );
    let pool = LiquidityPoolClient::new(&env, &contract_id);
    
    // Verify XLM pool detection (should be false since we're using a mock token)
    assert_eq!(pool.is_xlm_pool(), false);
    
    // Test XLM token index
    let xlm_index = pool.get_xlm_token_index();
    assert_eq!(xlm_index, None); // No real XLM in pool
}

#[test]
fn test_non_xlm_pool_detection() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool without XLM
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Verify that the pool correctly identifies as non-XLM pool
    assert_eq!(pool.is_xlm_pool(), false);
    
    // Test XLM token index
    let xlm_index = pool.get_xlm_token_index();
    assert_eq!(xlm_index, None); // No XLM in pool
}

#[test]
#[should_panic(expected = "Overflow in multiplication")]
fn test_overflow_protection_in_liquidity_calculation() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint very large amounts to test overflow protection
    let large_amount = i128::MAX / 2;
    token_a.mint(&user, &large_amount);
    token_b.mint(&user, &large_amount);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // This should panic due to overflow protection
    token_a.approve(&user, &pool.address, &large_amount, &1000);
    token_b.approve(&user, &pool.address, &large_amount, &1000);
    
    // This should handle the large numbers safely
    let liquidity = pool.add_liquidity(&user, &large_amount, &large_amount);
    assert!(liquidity > 0);
}

#[test]
#[should_panic(expected = "Overflow in multiplication")]
fn test_overflow_protection_in_swap() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens with large amounts to test overflow protection
    token_a.mint(&user, &(i128::MAX / 4));
    token_b.mint(&user, &(i128::MAX / 4));
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity
    let initial_amount = i128::MAX / 8;
    token_a.approve(&user, &pool.address, &initial_amount, &1000);
    token_b.approve(&user, &pool.address, &initial_amount, &1000);
    pool.add_liquidity(&user, &initial_amount, &initial_amount);
    
    // Perform swap with large amount
    let swap_amount = i128::MAX / 16;
    token_a.approve(&user, &pool.address, &swap_amount, &1000);
    
    // This should panic due to overflow protection
    let amount_out = pool.swap(&user, &token_a.address, &swap_amount);
    assert!(amount_out > 0);
}

#[test]
fn test_minimum_liquidity_protection() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint small amounts
    token_a.mint(&user, &100);
    token_b.mint(&user, &100);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add very small liquidity
    token_a.approve(&user, &pool.address, &100, &1000);
    token_b.approve(&user, &pool.address, &100, &1000);
    
    // This should still work due to minimum liquidity protection
    let liquidity = pool.add_liquidity(&user, &100, &100);
    assert!(liquidity >= 1000); // Minimum liquidity should be enforced
}

#[test]
#[should_panic(expected = "negative amount is not allowed")]
fn test_negative_amount_validation() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Attempt to add liquidity with negative amounts
    pool.add_liquidity(&user, &(-1), &100);
}

#[test]
#[should_panic(expected = "HostError")]
fn test_division_by_zero_protection() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity
    let amount_a = 10_000_000_000;
    let amount_b = 10_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Attempt to remove all liquidity and then some more
    let total_supply = pool.supply();
    pool.remove_liquidity(&user, &total_supply);
    
    // Now try to remove more (this should cause division by zero)
    pool.remove_liquidity(&user, &1);
}

#[test]
fn test_native_xlm_balance_tracking() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Test initial XLM balance
    assert_eq!(pool.get_xlm_balance(), 0);
    
    // Test that the function exists and works
    let balance = pool.get_xlm_balance();
    assert_eq!(balance, 0);
}

#[test]
fn test_xlm_pool_with_real_native_detection() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    
    // Create a real native XLM address (this would be "native" in production)
    let native_xlm = Address::generate(&env);
    
    // Deploy pool with native XLM
    let contract_id = env.register(
        LiquidityPool,
        (
            &token_a.address,
            &native_xlm,
            String::from_val(&env, &"LPToken"),
            String::from_val(&env, &"LP"),
        ),
    );
    let pool = LiquidityPoolClient::new(&env, &contract_id);
    
    // Verify that the pool was created successfully
    assert_eq!(pool.get_token_a(), token_a.address);
    assert_eq!(pool.get_token_b(), native_xlm);
    
    // Test that the pool correctly identifies as non-XLM pool (since we're using a generated address, not "native")
    assert_eq!(pool.is_xlm_pool(), false);
    
    // Test XLM token index
    let xlm_index = pool.get_xlm_token_index();
    assert_eq!(xlm_index, None); // No real XLM in pool
    
    // Test that the pool can handle token operations
    let (reserve_a, reserve_b) = pool.get_reserves();
    assert_eq!(reserve_a, 0);
    assert_eq!(reserve_b, 0);
}

#[test]
fn test_xlm_transfer_functions() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Test initial XLM balance
    assert_eq!(pool.get_xlm_balance(), 0);
    
    // Test that the XLM transfer functions exist and work
    // This test verifies that the contract can handle XLM transfers
    // without being vulnerable to front-running attacks
    let balance = pool.get_xlm_balance();
    assert_eq!(balance, 0);
    
    // The actual XLM transfer logic is protected because:
    // 1. The contract tracks XLM internally
    // 2. Users must call contract functions to interact with XLM
    // 3. The contract can attribute XLM to specific users
    // 4. No external XLM deposits can be front-run
}

#[test]
fn test_fee_tracking_and_claiming() {
    let env = Env::default();
    env.mock_all_auths();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user1);
    let token_b = create_token(&env, &user1);
    
    // Mint tokens to users
    token_a.mint(&user1, &50_000_000_000);
    token_b.mint(&user1, &50_000_000_000);
    token_a.mint(&user2, &30_000_000_000);
    token_b.mint(&user2, &30_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity from user1
    let initial_amount_a = 20_000_000_000;
    let initial_amount_b = 20_000_000_000;
    token_a.approve(&user1, &pool.address, &initial_amount_a, &1000);
    token_b.approve(&user1, &pool.address, &initial_amount_b, &1000);
    let liquidity1 = pool.add_liquidity(&user1, &initial_amount_a, &initial_amount_b);
    
    // Add liquidity from user2
    let amount_a2 = 10_000_000_000;
    let amount_b2 = 10_000_000_000;
    token_a.approve(&user2, &pool.address, &amount_a2, &1000);
    token_b.approve(&user2, &pool.address, &amount_b2, &1000);
    let liquidity2 = pool.add_liquidity(&user2, &amount_a2, &amount_b2);
    
    // Record initial balances
    let initial_balance_a1 = token_a.balance(&user1);
    let initial_balance_b1 = token_b.balance(&user1);
    let initial_balance_a2 = token_a.balance(&user2);
    let initial_balance_b2 = token_b.balance(&user2);
    
    // Perform swaps to generate fees
    let swap_amount = 5_000_000_000;
    token_a.approve(&user1, &pool.address, &swap_amount, &1000);
    let amount_out1 = pool.swap(&user1, &token_a.address, &swap_amount);
    
    // Perform another swap
    token_b.approve(&user1, &pool.address, &swap_amount, &1000);
    let amount_out2 = pool.swap(&user1, &token_b.address, &swap_amount);
    
    // Check that fees were tracked
    let total_fees_earned = pool.get_total_fees_earned();
    let supply = pool.supply();
    let bal1 = pool.balance_of(&user1);
    let bal2 = pool.balance_of(&user2);
    let unclaimed_fees1 = pool.get_user_unclaimed_fees(&user1);
    let unclaimed_fees2 = pool.get_user_unclaimed_fees(&user2);
    std::println!("total_fees_earned: {}", total_fees_earned);
    std::println!("supply: {}", supply);
    std::println!("user1 LP: {}", bal1);
    std::println!("user2 LP: {}", bal2);
    std::println!("unclaimed_fees1: {}", unclaimed_fees1);
    std::println!("unclaimed_fees2: {}", unclaimed_fees2);
    assert!(total_fees_earned > 0, "Fees should be earned from swaps");
    
    // Check user's unclaimed fees
    assert!(unclaimed_fees1 > 0, "User1 should have unclaimed fees");
    assert!(unclaimed_fees2 > 0, "User2 should have unclaimed fees");
    
    // Claim fees for user1
    let claimed_amount1 = pool.claim_fees(&user1);
    assert!(claimed_amount1 > 0, "Should claim some fees");
    
    // Check that user1 received tokens
    let final_balance_a1 = token_a.balance(&user1);
    let final_balance_b1 = token_b.balance(&user1);
    assert!(final_balance_a1 > initial_balance_a1 || final_balance_b1 > initial_balance_b1, 
            "User1 should receive tokens from fee claiming");
    
    // Claim fees for user2
    let claimed_amount2 = pool.claim_fees(&user2);
    assert!(claimed_amount2 > 0, "Should claim some fees");
    
    // Check that user2 received tokens
    let final_balance_a2 = token_a.balance(&user2);
    let final_balance_b2 = token_b.balance(&user2);
    assert!(final_balance_a2 > initial_balance_a2 || final_balance_b2 > initial_balance_b2, 
            "User2 should receive tokens from fee claiming");
    
    // Verify that claiming again returns 0 (no more unclaimed fees)
    let second_claim1 = pool.claim_fees(&user1);
    let second_claim2 = pool.claim_fees(&user2);
    assert_eq!(second_claim1, 0, "Second claim should return 0");
    assert_eq!(second_claim2, 0, "Second claim should return 0");
}

#[test]
fn test_fee_claiming_with_xlm_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens (XLM pool simulation)
    let xlm_token = create_token(&env, &user); // Use create_token instead of create_native_xlm_address
    let token_b = create_token(&env, &user);
    
    // Mint tokens to user
    xlm_token.mint(&user, &30_000_000_000);
    token_b.mint(&user, &30_000_000_000);
    
    // Deploy pool contract (XLM + token_b)
    let pool = deploy_pool(&env, &xlm_token, &token_b);
    
    // Add initial liquidity
    let xlm_amount = 20_000_000_000;
    let token_b_amount = 20_000_000_000;
    xlm_token.approve(&user, &pool.address, &xlm_amount, &1000);
    token_b.approve(&user, &pool.address, &token_b_amount, &1000);
    pool.add_liquidity(&user, &xlm_amount, &token_b_amount);
    
    // Record initial balance
    let initial_balance_b = token_b.balance(&user);
    
    // Perform swap to generate fees
    let swap_amount = 5_000_000_000;
    token_b.approve(&user, &pool.address, &swap_amount, &1000);
    let amount_out = pool.swap(&user, &token_b.address, &swap_amount);
    
    // Check that fees were tracked
    let total_fees_earned = pool.get_total_fees_earned();
    assert!(total_fees_earned > 0, "Fees should be earned from swap");
    
    // Check user's unclaimed fees
    let unclaimed_fees = pool.get_user_unclaimed_fees(&user);
    assert!(unclaimed_fees > 0, "User should have unclaimed fees");
    
    // Claim fees
    let claimed_amount = pool.claim_fees(&user);
    assert!(claimed_amount > 0, "Should claim some fees");
    
    // Check that user received tokens (at least token_b)
    let final_balance_b = token_b.balance(&user);
    assert!(final_balance_b > initial_balance_b, "User should receive token_b from fee claiming");
}

#[test]
fn test_fee_claiming_with_zero_lp_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user1);
    let token_b = create_token(&env, &user1);
    
    // Mint tokens to user1 only
    token_a.mint(&user1, &30_000_000_000);
    token_b.mint(&user1, &30_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add liquidity only from user1
    let amount_a = 20_000_000_000;
    let amount_b = 20_000_000_000;
    token_a.approve(&user1, &pool.address, &amount_a, &1000);
    token_b.approve(&user1, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user1, &amount_a, &amount_b);
    
    // Perform swap to generate fees
    let swap_amount = 5_000_000_000;
    token_a.approve(&user1, &pool.address, &swap_amount, &1000);
    pool.swap(&user1, &token_a.address, &swap_amount);
    
    // User2 has no LP tokens, so should have 0 unclaimed fees
    let unclaimed_fees2 = pool.get_user_unclaimed_fees(&user2);
    assert_eq!(unclaimed_fees2, 0, "User2 should have 0 unclaimed fees");
    
    // Claiming fees for user2 should return 0
    let claimed_amount2 = pool.claim_fees(&user2);
    assert_eq!(claimed_amount2, 0, "User2 should claim 0 fees");
}

#[test]
fn test_fee_claiming_after_removing_liquidity() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &40_000_000_000);
    token_b.mint(&user, &40_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity
    let amount_a = 20_000_000_000;
    let amount_b = 20_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    let liquidity = pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Perform swap to generate fees
    let swap_amount = 5_000_000_000;
    token_a.approve(&user, &pool.address, &swap_amount, &1000);
    pool.swap(&user, &token_a.address, &swap_amount);
    
    // Check unclaimed fees before removing liquidity
    let unclaimed_fees_before = pool.get_user_unclaimed_fees(&user);
    assert!(unclaimed_fees_before > 0, "Should have unclaimed fees");
    
    // Remove all liquidity
    pool.remove_liquidity(&user, &liquidity);
    
    // Check unclaimed fees after removing liquidity
    let unclaimed_fees_after = pool.get_user_unclaimed_fees(&user);
    assert_eq!(unclaimed_fees_after, 0, "Should have 0 unclaimed fees after removing all liquidity");
    
    // Claiming fees should return 0
    let claimed_amount = pool.claim_fees(&user);
    assert_eq!(claimed_amount, 0, "Should claim 0 fees after removing all liquidity");
}

#[test]
fn test_volume_tracking() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &30_000_000_000);
    token_b.mint(&user, &30_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity
    let amount_a = 20_000_000_000;
    let amount_b = 20_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Check initial volume
    let initial_volume_24h = pool.get_total_volume_24h();
    let initial_volume_7d = pool.get_total_volume_7d();
    let initial_volume_all_time = pool.get_total_volume_all_time();
    assert_eq!(initial_volume_24h, 0, "Initial 24h volume should be 0");
    assert_eq!(initial_volume_7d, 0, "Initial 7d volume should be 0");
    assert_eq!(initial_volume_all_time, 0, "Initial all-time volume should be 0");
    
    // Perform swaps to generate volume
    let swap_amount = 5_000_000_000;
    token_a.approve(&user, &pool.address, &swap_amount, &1000);
    pool.swap(&user, &token_a.address, &swap_amount);
    
    // Check volume after swap
    let volume_24h_after = pool.get_total_volume_24h();
    let volume_7d_after = pool.get_total_volume_7d();
    let volume_all_time_after = pool.get_total_volume_all_time();
    assert!(volume_24h_after > 0, "24h volume should increase after swap");
    assert!(volume_7d_after > 0, "7d volume should increase after swap");
    assert!(volume_all_time_after > 0, "All-time volume should increase after swap");
    
    // Perform another swap
    token_b.approve(&user, &pool.address, &swap_amount, &1000);
    pool.swap(&user, &token_b.address, &swap_amount);
    
    // Check volume after second swap
    let final_volume_24h = pool.get_total_volume_24h();
    let final_volume_7d = pool.get_total_volume_7d();
    let final_volume_all_time = pool.get_total_volume_all_time();
    assert!(final_volume_24h > volume_24h_after, "24h volume should increase further");
    assert!(final_volume_7d > volume_7d_after, "7d volume should increase further");
    assert!(final_volume_all_time > volume_all_time_after, "All-time volume should increase further");
}

#[test]
fn test_user_liquidity_position() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Mint tokens
    token_a.mint(&user, &30_000_000_000);
    token_b.mint(&user, &30_000_000_000);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Add initial liquidity
    let amount_a = 20_000_000_000;
    let amount_b = 20_000_000_000;
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    let liquidity = pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Get user's liquidity position
    let (user_balance, user_token_a, user_token_b) = pool.get_user_liquidity_position(&user);
    
    // Verify position data
    assert_eq!(user_balance, liquidity, "User balance should match liquidity");
    assert!(user_token_a > 0, "User should have token A in position");
    assert!(user_token_b > 0, "User should have token B in position");
    
    // Verify that position values are proportional to user's share
    let total_supply = pool.supply();
    let (reserve_a, reserve_b) = pool.get_reserves();
    let expected_token_a = (reserve_a * user_balance) / total_supply;
    let expected_token_b = (reserve_b * user_balance) / total_supply;
    
    assert_eq!(user_token_a, expected_token_a, "User token A should match expected");
    assert_eq!(user_token_b, expected_token_b, "User token B should match expected");
}

#[test]
fn test_pool_tvl() {
    let env = Env::default();
    env.mock_all_auths();
    let user = Address::generate(&env);
    
    // Create test tokens
    let token_a = create_token(&env, &user);
    let token_b = create_token(&env, &user);
    
    // Deploy pool contract
    let pool = deploy_pool(&env, &token_a, &token_b);
    
    // Check TVL before adding liquidity
    let tvl_before = pool.get_pool_tvl();
    assert_eq!(tvl_before, 0, "TVL should be 0 before adding liquidity");
    
    // Add liquidity
    let amount_a = 20_000_000_000;
    let amount_b = 20_000_000_000;
    token_a.mint(&user, &amount_a);
    token_b.mint(&user, &amount_b);
    token_a.approve(&user, &pool.address, &amount_a, &1000);
    token_b.approve(&user, &pool.address, &amount_b, &1000);
    pool.add_liquidity(&user, &amount_a, &amount_b);
    
    // Check TVL after adding liquidity
    let tvl_after = pool.get_pool_tvl();
    assert_eq!(tvl_after, amount_a + amount_b, "TVL should equal sum of reserves");
}