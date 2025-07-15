#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, BytesN, Env, IntoVal, Map, String, Val, Vec
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    PoolWasmHash,
    DeployedPools(Address, Address),
    AllPools, // Track all deployed pools for global calculations
}

#[contract]
pub struct PoolFactory;
#[contractimpl]
impl PoolFactory {

    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Set the pool contract Wasm hash (admin only)
    pub fn update_pool_wasm_hash(env: Env, admin_addr: Address, new_hash: BytesN<32>) {
        let admin = env.storage().instance().get::<_, Address>(&DataKey::Admin).expect("not set");
        assert!(admin == admin_addr, "Unauthorized");
        admin.require_auth();
        env.storage().instance().set(&DataKey::PoolWasmHash, &new_hash);
    }

    /// Get the pool contract Wasm hash
    pub fn get_pool_wasm_hash(env: Env) -> BytesN<32> {
        env.storage().instance().get(&DataKey::PoolWasmHash).expect("not set")
    }

    /// Deploy a new pool for a token pair, revert if already exists
    pub fn create_pool(
        env: Env,
        token_a: Address,
        token_b: Address,
        lp_token_name: String,
        lp_token_symbol: String,
        salt: BytesN<32>,
    ) -> Address {
        assert!(token_a != token_b, "Tokens must be different");
        // Sort addresses for uniqueness
        // let (token_0, token_1) = if token_a < token_b {
        //     (token_a.clone(), token_b.clone())
        // } else {
        //     (token_b.clone(), token_a.clone())
        // };
        let key = DataKey::DeployedPools(token_a.clone(), token_b.clone());
        if let Some(addr) = env.storage().instance().get::<_, Address>(&key) {
            panic!("Pool already exists for pair");
        }
        let wasm_hash = env
            .storage()
            .instance()
            .get::<_, BytesN<32>>(&DataKey::PoolWasmHash)
            .expect("Wasm hash not set");
        // // Deploy contract
        let pool_addr = env
            .deployer()
            .with_address(env.current_contract_address(), salt)
            .deploy_v2(wasm_hash, (
                token_a,
                token_b,
                lp_token_name,
                lp_token_symbol,
            ));
        // // Store mapping
        env.storage().instance().set(&key, &pool_addr);
        
        // Track pool in global list
        let mut all_pools = env.storage().instance().get::<_, Vec<Address>>(&DataKey::AllPools).unwrap_or(vec![&env]);
        all_pools.push_back(pool_addr.clone());
        env.storage().instance().set(&DataKey::AllPools, &all_pools);
        
        pool_addr
        // token_a
    }

    /// Get the pool address for a token pair, or None if not exists
    pub fn get_pool(env: Env, token_a: Address, token_b: Address) -> Option<Address> {
        // let (token_0, token_1) = if token_a < token_b {
        //     (token_a, token_b)
        // } else {
        //     (token_b, token_a)
        // };
        let key = DataKey::DeployedPools(token_a, token_b);
        env.storage().instance().get(&key)
    }

    /// Get all deployed pools
    pub fn get_all_pools(env: Env) -> Vec<Address> {
        env.storage().instance().get(&DataKey::AllPools).unwrap_or(vec![&env])
    }

    /// Get total number of pools
    pub fn get_pool_count(env: Env) -> u32 {
        let all_pools = Self::get_all_pools(env);
        all_pools.len() as u32
    }
}


mod test;