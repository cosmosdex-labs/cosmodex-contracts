#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, Env, String, Vec, Symbol,
    token::{self, Interface as _},
};
use soroban_token_sdk::TokenUtils;

// Constants
const FEE_BPS: i128 = 30; // 0.3%
const BPS_DENOMINATOR: i128 = 10000;

#[derive(Clone)]
#[contracttype]
pub struct PoolInfo {
    pub token_a: Address,
    pub token_b: Address,
    pub reserve_a: i128,
    pub reserve_b: i128,
}

#[contract]
pub struct LiquidityPool;

#[contractimpl]
impl LiquidityPool {
    
    pub fn __constructor(
        e: Env,
        token_a: Address,
        token_b: Address,
        lp_token_name: String,
        lp_token_symbol: String,
    ) {
        // Store pool info
        let pool_info = PoolInfo {
            token_a: token_a.clone(),
            token_b: token_b.clone(),
            reserve_a: 0,
            reserve_b: 0,
        };
        e.storage().instance().set(&symbol_short!("pool"), &pool_info);

        // Initialize LP token
        let admin = e.current_contract_address();
        e.storage().instance().set(&symbol_short!("admin"), &admin);
        e.storage().instance().set(&symbol_short!("name"), &lp_token_name);
        e.storage().instance().set(&symbol_short!("symbol"), &lp_token_symbol);
        e.storage().instance().set(&symbol_short!("decimals"), &18u32);
    }

    pub fn add_liquidity(e: Env, caller: Address, amount_a: i128, amount_b: i128) -> i128 {
        caller.require_auth();
        let mut pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();

        // Check for proportionality if reserves are not zero
        if pool_info.reserve_a > 0 || pool_info.reserve_b > 0 {
            assert!(
                amount_a * pool_info.reserve_b == amount_b * pool_info.reserve_a,
                "Amounts must be proportional"
            );
        }

        // Transfer tokens from user to pool
        let token_a_client = token::Client::new(&e, &pool_info.token_a);
        let token_b_client = token::Client::new(&e, &pool_info.token_b);

        token_a_client.transfer_from(&e.current_contract_address(), &caller, &e.current_contract_address(), &amount_a);
        token_b_client.transfer_from(&e.current_contract_address(), &caller, &e.current_contract_address(), &amount_b);

        // Calculate liquidity shares to mint
        let liquidity = sqrt(amount_a * amount_b);
        assert!(liquidity > 0, "Insufficient liquidity minted");
        let e_clone = e.clone();
        // Mint LP tokens to caller
        Self::mint_lp_tokens(e, caller, liquidity);

        // Update reserves
        pool_info.reserve_a += amount_a;
        pool_info.reserve_b += amount_b;
        e_clone.storage().instance().set(&symbol_short!("pool"), &pool_info);

        liquidity
    }

    pub fn remove_liquidity(e: Env, caller: Address, liquidity: i128) -> (i128, i128) {
        caller.require_auth();
        let mut pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        let total_supply = Self::total_supply(e.clone());
    
        assert!(total_supply > 0, "No liquidity in pool");
        assert!(liquidity > 0, "Liquidity must be > 0");
        assert!(
            liquidity <= Self::balance(e.clone(), caller.clone()),
            "Insufficient LP tokens"
        );
    
        // Calculate amounts of tokens to return
        let amount_a = (liquidity * pool_info.reserve_a) / total_supply;
        let amount_b = (liquidity * pool_info.reserve_b) / total_supply;
    
        assert!(amount_a > 0 || amount_b > 0, "Insufficient liquidity burned");
    
        // Burn LP tokens from caller
        Self::burn_lp_tokens(&e, &caller, liquidity);
    
        // Transfer tokens from pool to user
        let token_a_client = token::Client::new(&e, &pool_info.token_a);
        let token_b_client = token::Client::new(&e, &pool_info.token_b);
    
        token_a_client.transfer(&e.current_contract_address(), &caller, &amount_a);
        token_b_client.transfer(&e.current_contract_address(), &caller, &amount_b);
    
        // Update reserves
        pool_info.reserve_a -= amount_a;
        pool_info.reserve_b -= amount_b;
        e.storage().instance().set(&symbol_short!("pool"), &pool_info);
    
        (amount_a, amount_b)
    }

    pub fn swap(e: Env, caller: Address, input_token: Address, amount_in: i128) -> i128 {
        caller.require_auth();
        let mut pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();

        assert!(amount_in > 0, "Amount in must be > 0");
        assert!(
            input_token == pool_info.token_a || input_token == pool_info.token_b,
            "Invalid token address"
        );

        let is_token_a_in = input_token == pool_info.token_a;
        let (token_in, token_out, reserve_in, reserve_out) = if is_token_a_in {
            (
                pool_info.token_a.clone(),
                pool_info.token_b.clone(),
                pool_info.reserve_a,
                pool_info.reserve_b,
            )
        } else {
            (
                pool_info.token_b.clone(),
                pool_info.token_a.clone(),
                pool_info.reserve_b,
                pool_info.reserve_a,
            )
        };

        let token_in_client = token::Client::new(&e, &token_in);
        let token_out_client = token::Client::new(&e, &token_out);

        // Transfer input tokens from user to pool
        token_in_client.transfer_from(&e.current_contract_address(), &caller, &e.current_contract_address(), &amount_in);

        // Calculate amount after fee
        let amount_in_with_fee = amount_in * (BPS_DENOMINATOR - FEE_BPS) / BPS_DENOMINATOR;

        // Calculate amount out using constant product formula
        let amount_out = (reserve_out * amount_in_with_fee) / (reserve_in + amount_in_with_fee);

        assert!(amount_out > 0, "Insufficient output amount");
        assert!(amount_out <= reserve_out, "Insufficient pool reserves");

        // Transfer output tokens from pool to user
        token_out_client.transfer(&e.current_contract_address(), &caller, &amount_out);

        // Update reserves
        if is_token_a_in {
            pool_info.reserve_a += amount_in;
            pool_info.reserve_b -= amount_out;
        } else {
            pool_info.reserve_b += amount_in;
            pool_info.reserve_a -= amount_out;
        }
        e.storage().instance().set(&symbol_short!("pool"), &pool_info);

        amount_out
    }

    // View functions
    pub fn get_token_a(e: Env) -> Address {
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        pool_info.token_a
    }

    pub fn get_token_b(e: Env) -> Address {
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        pool_info.token_b
    }

    pub fn get_reserves(e: Env) -> (i128, i128) {
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        (pool_info.reserve_a, pool_info.reserve_b)
    }

    pub fn supply(e: Env) -> i128 {
        e.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0)
    }

    // Internal LP token functions
    fn total_supply(e: Env) -> i128 {
        e.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0)
    }

    fn mint_lp_tokens(e: Env, to: Address, amount: i128) {
        let balance = Self::balance(e.clone(), to.clone());
        let e_clone = e.clone();
        e_clone.storage().instance().set(&DataKey::Balance(to.clone()), &(balance + amount));
        let total = Self::total_supply(e.clone());
        let e_clone = e.clone();
        e_clone.storage().instance().set(&DataKey::TotalSupply, &(total + amount));
        TokenUtils::new(&e).events().mint(e.current_contract_address(), to, amount);
    }

    fn burn_lp_tokens(e: &Env, from: &Address, amount: i128) {
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "insufficient balance");
        e.storage().instance().set(&DataKey::Balance(from.clone()), &(balance - amount));
        let total = Self::total_supply(e.clone());
        e.storage().instance().set(&DataKey::TotalSupply, &(total - amount));
        TokenUtils::new(e).events().burn(from.clone(), amount);
    }
}

// LP Token implementation
#[contractimpl]
impl token::Interface for LiquidityPool {
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        e.storage()
            .instance()
            .get(&DataKey::Allowance(from, spender))
            .unwrap_or(0)
    }

    fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        from.require_auth();
        e.storage()
            .instance()
            .set(&DataKey::Allowance(from.clone(), spender.clone()), &amount);
        TokenUtils::new(&e).events().approve(from, spender, amount, expiration_ledger);
    }

    fn balance(e: Env, id: Address) -> i128 {
        e.storage()
            .instance()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "insufficient balance");
        let e_clone = e.clone();
        e_clone.storage()
            .instance()
            .set(&DataKey::Balance(from.clone()), &(balance - amount));
        let to_balance = Self::balance(e.clone(), to.clone());
        let e_clone = e.clone();
        e_clone.storage()
            .instance()
            .set(&DataKey::Balance(to.clone()), &(to_balance + amount));
        TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        let allowance = Self::allowance(e.clone(), from.clone(), spender.clone());
        assert!(allowance >= amount, "insufficient allowance");
        let e_clone = e.clone();
        e_clone.storage()
            .instance()
            .set(&DataKey::Allowance(from.clone(), spender.clone()), &(allowance - amount));
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "insufficient balance");
        let e_clone = e.clone();
        e_clone.storage()
            .instance()
            .set(&DataKey::Balance(from.clone()), &(balance - amount));
        let to_balance = Self::balance(e_clone.clone(), to.clone());
        e_clone.storage()
            .instance()
            .set(&DataKey::Balance(to.clone()), &(to_balance + amount));
        TokenUtils::new(&e).events().transfer(from, to, amount);
    }

    fn burn(e: Env, from: Address, amount: i128) {
        from.require_auth();
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "insufficient balance");
        let e_clone = e.clone();
        e_clone.storage()
            .instance()
            .set(&DataKey::Balance(from.clone()), &(balance - amount));
        TokenUtils::new(&e).events().burn(from, amount);
    }

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        let allowance = Self::allowance(e.clone(), from.clone(), spender.clone());
        assert!(allowance >= amount, "insufficient allowance");
        let e_clone = e.clone();
        e_clone.storage()
            .instance()
            .set(&DataKey::Allowance(from.clone(), spender.clone()), &(allowance - amount));
        let balance = Self::balance(e_clone.clone(), from.clone());
        assert!(balance >= amount, "insufficient balance");
        e_clone.storage()
            .instance()
            .set(&DataKey::Balance(from.clone()), &(balance - amount));
        TokenUtils::new(&e_clone).events().burn(from, amount);
    }

    fn decimals(e: Env) -> u32 {
        e.storage().instance().get(&symbol_short!("decimals")).unwrap()
    }

    fn name(e: Env) -> String {
        e.storage().instance().get(&symbol_short!("name")).unwrap()
    }

    fn symbol(e: Env) -> String {
        e.storage().instance().get(&symbol_short!("symbol")).unwrap()
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
}

// Helper function to calculate square root
fn sqrt(x: i128) -> i128 {
    if x == 0 {
        return 0;
    }
    let mut z = (x / 2) + 1;
    let mut y = x;
    while z < y {
        y = z;
        z = (x / z + z) / 2;
    }
    y
}


// pub use contract::{LiquidityPool, LiquidityPoolClient};
mod test;