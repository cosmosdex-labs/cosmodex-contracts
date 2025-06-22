#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, BytesN, Env, IntoVal, Map, String, Val, Vec
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    TokenWasmHash,
    DeployedTokens(Address, Address), // (token_address, Deployer) sorted
    AllDeployedTokens,
    TokenMetadata(Address),
}


#[contract]
pub struct TokenFactory;

#[contractimpl]
impl TokenFactory {
   
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

     /// Set the pool contract Wasm hash (admin only)
     pub fn update_pool_wasm_hash(env: Env, admin_addr: Address, new_hash: BytesN<32>) {
        let admin = env.storage().instance().get::<_, Address>(&DataKey::Admin).expect("not set");
        assert!(admin == admin_addr, "Unauthorized");
        admin.require_auth();
        env.storage().instance().set(&DataKey::TokenWasmHash, &new_hash);
    }

     /// Get the pool contract Wasm hash
    pub fn get_pool_wasm_hash(env: Env) -> BytesN<32> {
        env.storage().instance().get(&DataKey::TokenWasmHash).expect("not set")
    }
    
    pub fn create_token(env: Env, admin_addr: Address, token_name: String, token_symbol: String, token_decimals: u32, token_supply: i128, token_owner: Address, token_metadata: String, salt: BytesN<32>) -> Address {
        let wasm_hash = env
            .storage()
            .instance()
            .get::<_, BytesN<32>>(&DataKey::TokenWasmHash)
            .expect("Wasm hash not set");
        let token_addr = env.deployer().with_address(env.current_contract_address(), salt).deploy_v2(wasm_hash, (admin_addr, token_decimals, token_name, token_symbol, token_supply, token_owner));
        env.storage().instance().set(&DataKey::DeployedTokens(token_addr.clone(), env.current_contract_address()), &true);
        // Add the token to the list of all deployed tokens
    let mut tokens = env.storage().instance().get(&DataKey::AllDeployedTokens)
    .unwrap_or_else(|| Vec::new(&env));

    tokens.push_back(token_addr.clone());
    env.storage().instance().set(&DataKey::AllDeployedTokens, &tokens);
    env.storage().instance().set(&DataKey::TokenMetadata(token_addr.clone()), &token_metadata);
        token_addr
    }

    pub fn get_all_deployed_tokens(env: Env) -> Vec<Address> {
        env.storage().instance().get(&DataKey::AllDeployedTokens)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_token_metadata(env: Env, token_addr: Address) -> String {
        env.storage().instance().get(&DataKey::TokenMetadata(token_addr))
            .expect("Token metadata not set")
    }

}

mod test;
