#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, String,
    token::{self},
};

// Constants
const FEE_BPS: i128 = 30; // 0.3%
const BPS_DENOMINATOR: i128 = 10000;
const MINIMUM_LIQUIDITY: i128 = 1000; // Minimum liquidity to prevent division by zero

// XLM Native Asset Address
const XLM_ASSET: &str = "native";

// Helper function to check if an address represents native XLM
fn is_native_xlm(address: &Address) -> bool {
    let address_str = address.to_string();
    // Check against both the actual native XLM contract address and the "native" string
    address_str == String::from_str(&Env::default(), "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC") ||
    address_str == String::from_str(&Env::default(), "native")
}

// Overflow protection functions
fn checked_add(a: i128, b: i128) -> i128 {
    a.checked_add(b).expect("Overflow in addition")
}

fn checked_sub(a: i128, b: i128) -> i128 {
    a.checked_sub(b).expect("Underflow in subtraction")
}

fn checked_mul(a: i128, b: i128) -> i128 {
    a.checked_mul(b).expect("Overflow in multiplication")
}

fn checked_div(a: i128, b: i128) -> i128 {
    if b == 0 {
        panic!("Division by zero");
    }
    a.checked_div(b).expect("Overflow in division")
}

#[derive(Clone)]
#[contracttype]
pub struct PoolInfo {
    pub token_a: Address,
    pub token_b: Address,
    pub reserve_a: i128,
    pub reserve_b: i128,
    pub is_xlm_pool: bool, // Flag to indicate if one of the tokens is XLM
    pub xlm_token_index: Option<i32>, // 0 for token_a, 1 for token_b, None if no XLM
}

#[derive(Clone)]
#[contracttype]
pub struct FeeTracker {
    pub total_fees_earned: i128,      // Total fees earned by the pool
    pub fees_per_lp_token: i128,      // Fees per LP token (scaled by 1e18)
    pub last_update_ledger: u32,      // Last ledger when fees were updated
}

#[derive(Clone)]
#[contracttype]
pub struct VolumeTracker {
    pub total_volume_24h: i128,       // 24-hour volume
    pub total_volume_7d: i128,        // 7-day volume
    pub total_volume_all_time: i128,  // All-time volume
    pub last_swap_ledger: u32,        // Last ledger when a swap occurred
}

#[contract]
pub struct LiquidityPool;

#[contractimpl]
impl LiquidityPool {
    
    // Native XLM balance management functions
    fn get_native_xlm_balance(e: &Env) -> i128 {
        e.storage().instance().get(&DataKey::NativeXlmBalance).unwrap_or(0)
    }

    fn set_native_xlm_balance(e: &Env, amount: i128) {
        e.storage().instance().set(&DataKey::NativeXlmBalance, &amount);
    }

    fn add_native_xlm_balance(e: &Env, amount: i128) {
        let current_balance = Self::get_native_xlm_balance(e);
        let new_balance = checked_add(current_balance, amount);
        Self::set_native_xlm_balance(e, new_balance);
    }

    fn subtract_native_xlm_balance(e: &Env, amount: i128) {
        let current_balance = Self::get_native_xlm_balance(e);
        if current_balance < amount {
            panic!("Insufficient native XLM balance");
        }
        let new_balance = checked_sub(current_balance, amount);
        Self::set_native_xlm_balance(e, new_balance);
    }

    // Native XLM transfer functions
    fn transfer_native_xlm_to_user(e: &Env, _to: &Address, amount: i128) {
        // Transfer native XLM from contract to user
        // In Soroban, native XLM transfers are handled by the protocol
        // We'll use the contract's internal balance tracking
        Self::subtract_native_xlm_balance(e, amount);
        
    }

    fn transfer_native_xlm_from_user(e: &Env, _from: &Address, amount: i128) {
        // Transfer native XLM from user to contract
        // In Soroban, the user must authorize the contract to spend their XLM
        // We'll track the incoming XLM in the contract's balance
        Self::add_native_xlm_balance(e, amount);
        
    }

    fn check_nonnegative_amount(amount: i128) {
        if amount < 0 {
            panic!("negative amount is not allowed: {}", amount)
        }
    }

    // Fee tracking functions
    fn get_fee_tracker(e: &Env) -> FeeTracker {
        e.storage().instance().get(&DataKey::FeeTracker).unwrap_or(FeeTracker {
            total_fees_earned: 0,
            fees_per_lp_token: 0,
            last_update_ledger: 0,
        })
    }

    fn set_fee_tracker(e: &Env, tracker: &FeeTracker) {
        e.storage().instance().set(&DataKey::FeeTracker, tracker);
    }

    fn update_fees(e: &Env, fee_amount: i128) {
        let mut tracker = Self::get_fee_tracker(e);
        let current_ledger = e.ledger().sequence();
        tracker.total_fees_earned = checked_add(tracker.total_fees_earned, fee_amount);
        let total_supply = Self::total_supply(e.clone());
        if total_supply > 0 {
            // Use high precision for per-LP-token fee math (1e18 scaling)
            let fee_increment = checked_div(checked_mul(fee_amount, 1_000_000_000_000_000_000), total_supply);
            tracker.fees_per_lp_token = checked_add(tracker.fees_per_lp_token, fee_increment);
        }
        tracker.last_update_ledger = current_ledger;
        Self::set_fee_tracker(e, &tracker);
    }

    fn get_user_last_fees_per_lp_token(e: &Env, user: &Address) -> i128 {
        e.storage().instance().get(&DataKey::UserLastFeesPerLpToken(user.clone())).unwrap_or(0)
    }
    fn set_user_last_fees_per_lp_token(e: &Env, user: &Address, value: i128) {
        e.storage().instance().set(&DataKey::UserLastFeesPerLpToken(user.clone()), &value);
    }

    // Calculate user's unclaimed fees using per-LP-token fee accounting
    fn calculate_user_unclaimed_fees(e: &Env, user: &Address) -> i128 {
        let tracker = Self::get_fee_tracker(e);
        let user_balance = Self::balance_of(e.clone(), user.clone());
        if user_balance == 0 {
            return 0;
        }
        let last_claimed = Self::get_user_last_fees_per_lp_token(e, user);
        let delta = checked_sub(tracker.fees_per_lp_token, last_claimed);
        // Divide by 1e18 to get actual fee amount
        checked_div(checked_mul(user_balance, delta), 1_000_000_000_000_000_000)
    }

    // Volume tracking functions
    fn get_volume_tracker(e: &Env) -> VolumeTracker {
        e.storage().instance().get(&DataKey::VolumeTracker).unwrap_or(VolumeTracker {
            total_volume_24h: 0,
            total_volume_7d: 0,
            total_volume_all_time: 0,
            last_swap_ledger: 0,
        })
    }

    fn set_volume_tracker(e: &Env, tracker: &VolumeTracker) {
        e.storage().instance().set(&DataKey::VolumeTracker, tracker);
    }

    fn update_volume(e: &Env, volume_amount: i128) {
        let mut tracker = Self::get_volume_tracker(e);
        let current_ledger = e.ledger().sequence();
        
        // Update all-time volume
        tracker.total_volume_all_time = checked_add(tracker.total_volume_all_time, volume_amount);
        
        // For simplicity, we'll update 24h and 7d volume as all-time for now
        tracker.total_volume_24h = tracker.total_volume_all_time;
        tracker.total_volume_7d = tracker.total_volume_all_time;
        
        tracker.last_swap_ledger = current_ledger;
        Self::set_volume_tracker(e, &tracker);
    }

    // User fee tracking functions
    fn get_user_fees_claimed(e: &Env, user: &Address) -> i128 {
        e.storage().instance().get(&DataKey::UserFeesClaimed(user.clone())).unwrap_or(0)
    }

    fn set_user_fees_claimed(e: &Env, user: &Address, amount: i128) {
        e.storage().instance().set(&DataKey::UserFeesClaimed(user.clone()), &amount);
    }

    pub fn __constructor(
        e: Env,
        token_a: Address,
        token_b: Address,
        lp_token_name: String,
        lp_token_symbol: String,
    ) {
        // Determine if this is an XLM pool
        let is_xlm_pool = is_native_xlm(&token_a) || is_native_xlm(&token_b);
        let xlm_token_index = if is_native_xlm(&token_a) {
            Some(0)
        } else if is_native_xlm(&token_b) {
            Some(1)
        } else {
            None
        };

        let pool_info = PoolInfo {
            token_a: token_a.clone(),
            token_b: token_b.clone(),
            reserve_a: 0,
            reserve_b: 0,
            is_xlm_pool,
            xlm_token_index,
        };

        e.storage().instance().set(&symbol_short!("pool"), &pool_info);
        e.storage().instance().set(&symbol_short!("name"), &lp_token_name);
        e.storage().instance().set(&symbol_short!("symbol"), &lp_token_symbol);
        e.storage().instance().set(&symbol_short!("decimals"), &18u32);
    }

    pub fn add_liquidity(e: Env, caller: Address, amount_a: i128, amount_b: i128) -> i128 {
        caller.require_auth();
        Self::check_nonnegative_amount(amount_a);
        Self::check_nonnegative_amount(amount_b);
        
        let mut pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();

        // Check for proportionality if reserves are not zero
        if pool_info.reserve_a > 0 || pool_info.reserve_b > 0 {
            // Use checked arithmetic to prevent overflow
            let left_side = checked_mul(amount_a, pool_info.reserve_b);
            let right_side = checked_mul(amount_b, pool_info.reserve_a);
            
            // Allow for small rounding differences (tolerance of 1 unit or 0.001%)
            let difference = if left_side > right_side {
                checked_sub(left_side, right_side)
            } else {
                checked_sub(right_side, left_side)
            };
            
            // Calculate percentage difference for better tolerance
            let percentage_difference = if left_side > 0 {
                (difference * 10000) / left_side // Multiply by 10000 to get percentage with 2 decimal places
            } else {
                0
            };
            
            // Allow tolerance of 1 unit OR 0.001% (10 parts per million)
            assert!(difference <= 1 || percentage_difference <= 10, "Amounts must be proportional");
        }

        // Handle token transfers based on whether it's an XLM pool
        if pool_info.is_xlm_pool {
            Self::handle_xlm_liquidity_addition(&e, &caller, &pool_info, amount_a, amount_b);
        } else {
            // Standard token transfer for non-XLM pools
            let token_a_client = token::Client::new(&e, &pool_info.token_a);
            let token_b_client = token::Client::new(&e, &pool_info.token_b);

            token_a_client.transfer_from(&e.current_contract_address(), &caller, &e.current_contract_address(), &amount_a);
            token_b_client.transfer_from(&e.current_contract_address(), &caller, &e.current_contract_address(), &amount_b);
        }

        // Calculate liquidity shares to mint with overflow protection
        let liquidity = Self::calculate_liquidity(amount_a, amount_b);
        assert!(liquidity > 0, "Insufficient liquidity minted");
        
        let e_clone = e.clone();
        // Mint LP tokens to caller
        Self::mint_lp_tokens(e, caller, liquidity);

        // Update reserves with overflow protection
        pool_info.reserve_a = checked_add(pool_info.reserve_a, amount_a);
        pool_info.reserve_b = checked_add(pool_info.reserve_b, amount_b);
        e_clone.storage().instance().set(&symbol_short!("pool"), &pool_info);

        liquidity
    }

    // Handle XLM liquidity addition
    fn handle_xlm_liquidity_addition(e: &Env, caller: &Address, pool_info: &PoolInfo, amount_a: i128, amount_b: i128) {
        match pool_info.xlm_token_index {
            Some(0) => {
                // Token A is XLM, Token B is contract token
                let token_b_client = token::Client::new(e, &pool_info.token_b);
                
                // Transfer XLM from caller to contract
                Self::transfer_native_xlm_from_user(e, caller, amount_a);
                // Transfer contract token from caller to pool
                token_b_client.transfer_from(&caller, &e.current_contract_address(), &e.current_contract_address(), &amount_b);
            },
            Some(1) => {
                // Token A is contract token, Token B is XLM
                let token_a_client = token::Client::new(e, &pool_info.token_a);
                
                // Transfer contract token from caller to pool
                token_a_client.transfer_from(&caller, &e.current_contract_address(), &e.current_contract_address(), &amount_a);
                // Transfer XLM from caller to contract
                Self::transfer_native_xlm_from_user(e, caller, amount_b);
            },
            Some(_) => {
                panic!("Invalid XLM token index");
            },
            None => {
                panic!("XLM pool but no XLM token index found");
            }
        }
    }

    // Safe liquidity calculation with overflow protection
    fn calculate_liquidity(amount_a: i128, amount_b: i128) -> i128 {
        if amount_a == 0 || amount_b == 0 {
            return 0;
        }
        
        // Use checked multiplication to prevent overflow
        let product = checked_mul(amount_a, amount_b);
        let liquidity = sqrt(product);
        
        // Ensure minimum liquidity
        if liquidity < MINIMUM_LIQUIDITY {
            return MINIMUM_LIQUIDITY;
        }
        
        liquidity
    }

    pub fn remove_liquidity(e: Env, caller: Address, liquidity: i128) -> (i128, i128) {
        caller.require_auth();
        Self::check_nonnegative_amount(liquidity);
        
        let mut pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        let total_supply = Self::total_supply(e.clone());
    
        assert!(total_supply > 0, "No liquidity in pool");
        assert!(liquidity > 0, "Liquidity must be > 0");
        
        // Directly access storage instead of calling the interface method to avoid recursion
        let caller_balance = e.storage().instance().get(&DataKey::Balance(caller.clone())).unwrap_or(0);
        assert!(
            liquidity <= caller_balance,
            "Insufficient LP tokens"
        );
        
        // Ensure we don't try to remove more than what's available
        assert!(liquidity <= total_supply, "Cannot remove more than total supply");
        
        // Ensure reserves are not zero to prevent division issues
        assert!(pool_info.reserve_a > 0 || pool_info.reserve_b > 0, "Pool has no reserves");
    
        // Calculate amounts of tokens to return with overflow protection
        let amount_a = if pool_info.reserve_a > 0 {
            checked_div(checked_mul(liquidity, pool_info.reserve_a), total_supply)
        } else {
            0
        };
        
        let amount_b = if pool_info.reserve_b > 0 {
            checked_div(checked_mul(liquidity, pool_info.reserve_b), total_supply)
        } else {
            0
        };
    
        // Ensure we're not trying to remove more than the reserves
        assert!(amount_a <= pool_info.reserve_a, "Cannot remove more token A than available");
        assert!(amount_b <= pool_info.reserve_b, "Cannot remove more token B than available");
    
        
        // Burn LP tokens from caller
        Self::burn_lp_tokens(&e, &caller, liquidity);
        
    
        // Handle token transfers based on whether it's an XLM pool
        if pool_info.is_xlm_pool {
            Self::handle_xlm_liquidity_removal(&e, &caller, &pool_info, amount_a, amount_b);
        } else {
            // Standard token transfer for non-XLM pools
            let token_a_client = token::Client::new(&e, &pool_info.token_a);
            let token_b_client = token::Client::new(&e, &pool_info.token_b);
        
            // Only transfer if amounts are greater than 0
            if amount_a > 0 {
                token_a_client.transfer(&e.current_contract_address(), &caller, &amount_a);
            }
            if amount_b > 0 {
                token_b_client.transfer(&e.current_contract_address(), &caller, &amount_b);
            }
        }
        
    
        // Update reserves with overflow protection
        pool_info.reserve_a = checked_sub(pool_info.reserve_a, amount_a);
        pool_info.reserve_b = checked_sub(pool_info.reserve_b, amount_b);
        e.storage().instance().set(&symbol_short!("pool"), &pool_info);
        
    
        (amount_a, amount_b)
    }

    // Handle XLM liquidity removal
    fn handle_xlm_liquidity_removal(e: &Env, caller: &Address, pool_info: &PoolInfo, amount_a: i128, amount_b: i128) {
        match pool_info.xlm_token_index {
            Some(0) => {
                // Token A is XLM, Token B is contract token
                let token_b_client = token::Client::new(e, &pool_info.token_b);
                
                // Transfer contract token from pool to caller (only if amount > 0)
                if amount_b > 0 {
                    token_b_client.transfer(&e.current_contract_address(), caller, &amount_b);
                }
                // Transfer XLM from contract to caller (only if amount > 0)
                if amount_a > 0 {
                    Self::transfer_native_xlm_to_user(e, caller, amount_a);
                }
            },
            Some(1) => {
                // Token A is contract token, Token B is XLM
                let token_a_client = token::Client::new(e, &pool_info.token_a);
                
                // Transfer contract token from pool to caller (only if amount > 0)
                if amount_a > 0 {
                    token_a_client.transfer(&e.current_contract_address(), caller, &amount_a);
                }
                // Transfer XLM from contract to caller (only if amount > 0)
                if amount_b > 0 {
                    Self::transfer_native_xlm_to_user(e, caller, amount_b);
                }
            },
            Some(_) => {
                panic!("Invalid XLM token index");
            },
            None => {
                panic!("XLM pool but no XLM token index found");
            }
        }
    }

    pub fn swap(e: Env, caller: Address, input_token: Address, amount_in: i128) -> i128 {
        caller.require_auth();
        Self::check_nonnegative_amount(amount_in);
        
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

        // Handle input token transfer based on whether it's XLM
        if pool_info.is_xlm_pool && (is_native_xlm(&token_in) || is_native_xlm(&token_out)) {
            Self::handle_xlm_swap_input(&e, &caller, &token_in, &token_out, amount_in);
        } else {
            // Standard token transfer for non-XLM swaps
            let token_in_client = token::Client::new(&e, &token_in);
            token_in_client.transfer_from(&e.current_contract_address(), &caller, &e.current_contract_address(), &amount_in);
        }

        // Calculate amount after fee with overflow protection
        let amount_in_with_fee = checked_div(checked_mul(amount_in, checked_sub(BPS_DENOMINATOR, FEE_BPS)), BPS_DENOMINATOR);

        // Calculate amount out using constant product formula with overflow protection
        let numerator = checked_mul(reserve_out, amount_in_with_fee);
        let denominator = checked_add(reserve_in, amount_in_with_fee);
        let amount_out = checked_div(numerator, denominator);

        assert!(amount_out > 0, "Insufficient output amount");
        assert!(amount_out <= reserve_out, "Insufficient pool reserves");

        // Calculate and track fees
        let fee_amount = checked_sub(amount_in, amount_in_with_fee);
        Self::update_fees(&e, fee_amount);

        // Track volume
        Self::update_volume(&e, amount_in);

        // Handle output token transfer based on whether it's XLM
        if pool_info.is_xlm_pool && (is_native_xlm(&token_in) || is_native_xlm(&token_out)) {
            Self::handle_xlm_swap_output(&e, &caller, &token_in, &token_out, amount_out);
        } else {
            // Standard token transfer for non-XLM swaps
            let token_out_client = token::Client::new(&e, &token_out);
            token_out_client.transfer(&e.current_contract_address(), &caller, &amount_out);
        }

        // Update reserves with overflow protection
        if is_token_a_in {
            pool_info.reserve_a = checked_add(pool_info.reserve_a, amount_in);
            pool_info.reserve_b = checked_sub(pool_info.reserve_b, amount_out);
        } else {
            pool_info.reserve_b = checked_add(pool_info.reserve_b, amount_in);
            pool_info.reserve_a = checked_sub(pool_info.reserve_a, amount_out);
        }
        e.storage().instance().set(&symbol_short!("pool"), &pool_info);

        amount_out
    }

    // Handle XLM swap input
    fn handle_xlm_swap_input(e: &Env, caller: &Address, token_in: &Address, _token_out: &Address, amount_in: i128) {
        if is_native_xlm(token_in) {
            // Input is XLM - transfer from caller to contract
            Self::transfer_native_xlm_from_user(e, caller, amount_in);
        } else {
            // Input is contract token
            let token_in_client = token::Client::new(e, token_in);
            token_in_client.transfer_from(&caller, &e.current_contract_address(), &e.current_contract_address(), &amount_in);
        }
    }

    // Handle XLM swap output
    fn handle_xlm_swap_output(e: &Env, caller: &Address, _token_in: &Address, token_out: &Address, amount_out: i128) {
        if is_native_xlm(token_out) {
            // Output is XLM - transfer from contract to caller
            Self::transfer_native_xlm_to_user(e, caller, amount_out);
        } else {
            // Output is contract token
            let token_out_client = token::Client::new(e, token_out);
            token_out_client.transfer(&e.current_contract_address(), &caller, &amount_out);
        }
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

    pub fn is_xlm_pool(e: Env) -> bool {
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        pool_info.is_xlm_pool
    }

    pub fn get_xlm_token_index(e: Env) -> Option<i32> {
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        pool_info.xlm_token_index
    }

    pub fn get_xlm_balance(e: Env) -> i128 {
        Self::get_native_xlm_balance(&e)
    }

    pub fn supply(e: Env) -> i128 {
        Self::total_supply(e)
    }

    // New fee tracking methods
    pub fn get_total_fees_earned(e: Env) -> i128 {
        let tracker = Self::get_fee_tracker(&e);
        tracker.total_fees_earned
    }

    pub fn get_fees_per_lp_token(e: Env) -> i128 {
        let tracker = Self::get_fee_tracker(&e);
        tracker.fees_per_lp_token
    }

    pub fn get_user_unclaimed_fees(e: Env, user: Address) -> i128 {
        Self::calculate_user_unclaimed_fees(&e, &user)
    }

    pub fn claim_fees(e: Env, caller: Address) -> i128 {
        caller.require_auth();
        
        let unclaimed_fees = Self::calculate_user_unclaimed_fees(&e, &caller);
        if unclaimed_fees <= 0 {
            return 0;
        }
        
        // Get pool info
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        let total_supply = Self::total_supply(e.clone());
        let user_balance = Self::balance_of(e.clone(), caller.clone());
        
        // Calculate user's share of the pool
        let user_share = if total_supply > 0 {
            checked_div(checked_mul(user_balance, 10000), total_supply) // In basis points
        } else {
            0
        };
        
        // Calculate fee amounts for each token based on user's share
        let fee_tracker = Self::get_fee_tracker(&e);
        let total_fees_earned = fee_tracker.total_fees_earned;
        
        // Calculate user's share of fees for each token
        let user_fee_share_a = if total_supply > 0 {
            checked_div(checked_mul(pool_info.reserve_a, user_share), 10000)
        } else {
            0
        };
        
        let user_fee_share_b = if total_supply > 0 {
            checked_div(checked_mul(pool_info.reserve_b, user_share), 10000)
        } else {
            0
        };
        
        // Update claimed fees
        let current_claimed = Self::get_user_fees_claimed(&e, &caller);
        let new_claimed = checked_add(current_claimed, unclaimed_fees);
        Self::set_user_fees_claimed(&e, &caller, new_claimed);
        
        // Update user's last claimed fees_per_lp_token
        let tracker = Self::get_fee_tracker(&e);
        Self::set_user_last_fees_per_lp_token(&e, &caller, tracker.fees_per_lp_token);
        
        // Transfer fee tokens to user based on whether it's an XLM pool
        if pool_info.is_xlm_pool {
            Self::handle_xlm_fee_transfer(&e, &caller, &pool_info, user_fee_share_a, user_fee_share_b);
        } else {
            // Standard token transfer for non-XLM pools
            let token_a_client = token::Client::new(&e, &pool_info.token_a);
            let token_b_client = token::Client::new(&e, &pool_info.token_b);
            
            // Only transfer if amounts are greater than 0
            if user_fee_share_a > 0 {
                token_a_client.transfer(&e.current_contract_address(), &caller, &user_fee_share_a);
            }
            if user_fee_share_b > 0 {
                token_b_client.transfer(&e.current_contract_address(), &caller, &user_fee_share_b);
            }
        }
        
        // Return the total fee amount claimed
        checked_add(user_fee_share_a, user_fee_share_b)
    }

    // Handle XLM fee transfer
    fn handle_xlm_fee_transfer(e: &Env, caller: &Address, pool_info: &PoolInfo, fee_share_a: i128, fee_share_b: i128) {
        match pool_info.xlm_token_index {
            Some(0) => {
                // Token A is XLM, Token B is contract token
                let token_b_client = token::Client::new(e, &pool_info.token_b);
                
                // Transfer contract token fees (only if amount > 0)
                if fee_share_b > 0 {
                    token_b_client.transfer(&e.current_contract_address(), caller, &fee_share_b);
                }
                // Transfer XLM fees (only if amount > 0)
                if fee_share_a > 0 {
                    Self::transfer_native_xlm_to_user(e, caller, fee_share_a);
                }
            },
            Some(1) => {
                // Token A is contract token, Token B is XLM
                let token_a_client = token::Client::new(e, &pool_info.token_a);
                
                // Transfer contract token fees (only if amount > 0)
                if fee_share_a > 0 {
                    token_a_client.transfer(&e.current_contract_address(), caller, &fee_share_a);
                }
                // Transfer XLM fees (only if amount > 0)
                if fee_share_b > 0 {
                    Self::transfer_native_xlm_to_user(e, caller, fee_share_b);
                }
            },
            Some(_) => {
                panic!("Invalid XLM token index");
            },
            None => {
                panic!("XLM pool but no XLM token index found");
            }
        }
    }

    // New volume tracking methods
    pub fn get_total_volume_24h(e: Env) -> i128 {
        let tracker = Self::get_volume_tracker(&e);
        tracker.total_volume_24h
    }

    pub fn get_total_volume_7d(e: Env) -> i128 {
        let tracker = Self::get_volume_tracker(&e);
        tracker.total_volume_7d
    }

    pub fn get_total_volume_all_time(e: Env) -> i128 {
        let tracker = Self::get_volume_tracker(&e);
        tracker.total_volume_all_time
    }

    // Enhanced liquidity position methods
    pub fn get_user_liquidity_position(e: Env, user: Address) -> (i128, i128, i128) {
        let user_balance = Self::balance_of(e.clone(), user.clone());
        let total_supply = Self::total_supply(e.clone());
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        
        if total_supply == 0 || user_balance == 0 {
            return (0, 0, 0);
        }
        
        // Calculate user's share of the pool
        let user_share = checked_div(checked_mul(user_balance, 10000), total_supply); // In basis points
        
        // Calculate user's token amounts
        let user_token_a = checked_div(checked_mul(pool_info.reserve_a, user_balance), total_supply);
        let user_token_b = checked_div(checked_mul(pool_info.reserve_b, user_balance), total_supply);
        
        (user_balance, user_token_a, user_token_b)
    }

    pub fn get_pool_tvl(e: Env) -> i128 {
        let pool_info: PoolInfo = e.storage().instance().get(&symbol_short!("pool")).unwrap();
        // Simplified TVL calculation - in practice you'd use price oracles
        checked_add(pool_info.reserve_a, pool_info.reserve_b)
    }

    fn total_supply(e: Env) -> i128 {
        e.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0)
    }

    fn mint_lp_tokens(e: Env, to: Address, amount: i128) {
        let total_supply = Self::total_supply(e.clone());
        let new_total_supply = checked_add(total_supply, amount);
        e.storage().instance().set(&DataKey::TotalSupply, &new_total_supply);
        
        let balance = e.storage().instance().get(&DataKey::Balance(to.clone())).unwrap_or(0);
        let new_balance = checked_add(balance, amount);
        e.storage().instance().set(&DataKey::Balance(to), &new_balance);
    }

    fn burn_lp_tokens(e: &Env, from: &Address, amount: i128) {
        // Directly access storage to avoid recursive calls
        let total_supply = e.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0);
        let new_total_supply = checked_sub(total_supply, amount);
        e.storage().instance().set(&DataKey::TotalSupply, &new_total_supply);
        
        // Directly access storage instead of calling the interface method to avoid recursion
        let balance = e.storage().instance().get(&DataKey::Balance(from.clone())).unwrap_or(0);
        let new_balance = checked_sub(balance, amount);
        e.storage().instance().set(&DataKey::Balance(from.clone()), &new_balance);
    }

    pub fn balance_of(e: Env, id: Address) -> i128 {
        <LiquidityPool as token::Interface>::balance(e, id)
    }
}

impl token::Interface for LiquidityPool {
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        e.storage().instance().get(&DataKey::Allowance(from, spender)).unwrap_or(0)
    }

    fn approve(e: Env, from: Address, spender: Address, amount: i128, _expiration_ledger: u32) {
        from.require_auth();
        Self::check_nonnegative_amount(amount);
        e.storage().instance().set(&DataKey::Allowance(from, spender), &amount);
    }

    fn balance(e: Env, id: Address) -> i128 {
        e.storage().instance().get(&DataKey::Balance(id)).unwrap_or(0)
    }

    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        Self::check_nonnegative_amount(amount);
        
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "Insufficient balance");
        
        let new_from_balance = checked_sub(balance, amount);
        let to_balance = Self::balance(e.clone(), to.clone());
        let new_to_balance = checked_add(to_balance, amount);
        
        e.storage().instance().set(&DataKey::Balance(from), &new_from_balance);
        e.storage().instance().set(&DataKey::Balance(to), &new_to_balance);
    }

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        Self::check_nonnegative_amount(amount);
        
        let allowance = Self::allowance(e.clone(), from.clone(), spender.clone());
        assert!(allowance >= amount, "Insufficient allowance");
        
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "Insufficient balance");
        
        let new_allowance = checked_sub(allowance, amount);
        let new_from_balance = checked_sub(balance, amount);
        let to_balance = Self::balance(e.clone(), to.clone());
        let new_to_balance = checked_add(to_balance, amount);
        
        e.storage().instance().set(&DataKey::Allowance(from.clone(), spender), &new_allowance);
        e.storage().instance().set(&DataKey::Balance(from), &new_from_balance);
        e.storage().instance().set(&DataKey::Balance(to), &new_to_balance);
    }

    fn burn(e: Env, from: Address, amount: i128) {
        from.require_auth();
        Self::check_nonnegative_amount(amount);
        
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "Insufficient balance");
        
        let new_balance = checked_sub(balance, amount);
        let total_supply = Self::total_supply(e.clone());
        let new_total_supply = checked_sub(total_supply, amount);
        
        e.storage().instance().set(&DataKey::Balance(from), &new_balance);
        e.storage().instance().set(&DataKey::TotalSupply, &new_total_supply);
    }

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        Self::check_nonnegative_amount(amount);
        
        let allowance = Self::allowance(e.clone(), from.clone(), spender.clone());
        assert!(allowance >= amount, "Insufficient allowance");
        
        let balance = Self::balance(e.clone(), from.clone());
        assert!(balance >= amount, "Insufficient balance");
        
        let new_allowance = checked_sub(allowance, amount);
        let new_balance = checked_sub(balance, amount);
        let total_supply = Self::total_supply(e.clone());
        let new_total_supply = checked_sub(total_supply, amount);
        
        e.storage().instance().set(&DataKey::Allowance(from.clone(), spender), &new_allowance);
        e.storage().instance().set(&DataKey::Balance(from), &new_balance);
        e.storage().instance().set(&DataKey::TotalSupply, &new_total_supply);
    }

    fn decimals(e: Env) -> u32 {
        e.storage().instance().get(&symbol_short!("decimals")).unwrap_or(18)
    }

    fn name(e: Env) -> String {
        e.storage().instance().get(&symbol_short!("name")).unwrap_or(String::from_str(&e, "Liquidity Pool Token"))
    }

    fn symbol(e: Env) -> String {
        e.storage().instance().get(&symbol_short!("symbol")).unwrap_or(String::from_str(&e, "LP"))
    }
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
    NativeXlmBalance, // Track native XLM balance in the contract
    FeeTracker, // Track total fees earned and fees per LP token
    VolumeTracker, // Track total volume and last swap ledger
    UserFeesClaimed(Address), // Track user's total claimed fees (legacy, not used)
    UserLastFeesPerLpToken(Address), // Track user's last claimed fees_per_lp_token
}

fn sqrt(x: i128) -> i128 {
    if x <= 0 {
        return 0;
    }
    
    if x == 1 {
        return 1;
    }
    
    // Use binary search for better precision and overflow protection
    let mut left = 1;
    let mut right = x;
    let mut result = 0;
    
    while left <= right {
        let mid = left + (right - left) / 2;
        
        // Check if mid * mid <= x
        if mid <= x / mid {
            result = mid;
            left = mid + 1;
        } else {
            right = mid - 1;
        }
    }
    
    result
}

mod test;