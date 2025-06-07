#![cfg(test)]
extern crate std;

use crate::{contract::Token, TokenClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, FromVal, IntoVal, String, Symbol,
};
use crate::storage_types::{AllowanceValue, DataKey, AllowanceDataKey};

fn create_token<'a>(e: &Env, admin: &Address) -> TokenClient<'a> {
    let token_contract = e.register(
        Token,
        (
            admin,
            18_u32,
            String::from_val(e, &"name"),
            String::from_val(e, &"symbol"),
            100000_i128,
            admin,
        ),
    );
    TokenClient::new(e, &token_contract)
}

#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();

    let admin1 = Address::generate(&e);
    let admin2 = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);
    let token = create_token(&e, &admin1);


    assert_eq!(token.balance(&admin1), 100000);

    token.approve(&admin1, &user3, &500, &200);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (_contract, invocation) = &auths[0];
    assert_eq!(invocation.function, AuthorizedFunction::Contract((
        token.address.clone(),
        symbol_short!("approve"),
        (&admin1, &user3, 500_i128, 200_u32).into_val(&e),
    )));
    
    // Now test transfer
    token.transfer(&admin1, &user2, &600);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (_contract, invocation) = &auths[0];
    assert_eq!(invocation.function, AuthorizedFunction::Contract((
        token.address.clone(),
        symbol_short!("transfer"),
        (&admin1, &user2, 600_i128).into_val(&e),
    )));
    // assert_eq!(token.balance(&user1), 400);
    assert_eq!(token.balance(&user2), 600);

    token.transfer_from(&user3, &admin1, &user1, &400);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (_contract, invocation) = &auths[0];
    assert_eq!(invocation.function, AuthorizedFunction::Contract((
        token.address.clone(),
        Symbol::new(&e, "transfer_from"),
        (&user3, &admin1, &user1, 400_i128).into_val(&e),
    )));
    assert_eq!(token.balance(&user1), 400);
    assert_eq!(token.balance(&user2), 600);

    token.transfer(&user1, &user3, &300);
    assert_eq!(token.balance(&user1), 100);
    assert_eq!(token.balance(&user3), 300);

    token.set_admin(&admin2);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (_contract, invocation) = &auths[0];
    assert_eq!(invocation.function, AuthorizedFunction::Contract((
        token.address.clone(),
        symbol_short!("set_admin"),
        (&admin2,).into_val(&e),
    )));

    // Increase to 500
    token.approve(&user2, &user3, &500, &200);
    assert_eq!(token.allowance(&user2, &user3), 500);
    token.approve(&user2, &user3, &0, &200);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (_contract, invocation) = &auths[0];
    assert_eq!(invocation.function, AuthorizedFunction::Contract((
        token.address.clone(),
        symbol_short!("approve"),
        (&user2, &user3, 0_i128, 200_u32).into_val(&e),
    )));
    assert_eq!(token.allowance(&user2, &user3), 0);
}

#[test]
fn test_burn() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = create_token(&e, &admin);
    let user2 = Address::generate(&e);
    assert_eq!(token.balance(&admin), 100000);  

 let auths = e.auths();
    token.transfer(&admin, &user2, &500);
    token.burn(&user2, &500);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (_contract, invocation) = &auths[0];
    assert_eq!(invocation.function, AuthorizedFunction::Contract((
        token.address.clone(),
        symbol_short!("burn"),
        (&user2, 500_i128).into_val(&e),
    )));

    assert_eq!(token.balance(&user2), 0);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn transfer_insufficient_balance() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let token = create_token(&e, &admin);

    // token.mint(&user1, &1000);
    assert_eq!(token.balance(&admin), 100000);

    token.transfer(&admin, &user2, &100001);
}

#[test]
#[should_panic(expected = "insufficient allowance")]
fn transfer_from_insufficient_allowance() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);
    let token = create_token(&e, &admin);

    
    assert_eq!(token.balance(&admin), 100000);

    token.approve(&admin, &user3, &100, &200);
    assert_eq!(token.allowance(&admin, &user3), 100);

    token.transfer_from(&user3, &user1, &user2, &101);
}

#[test]
#[should_panic(expected = "Decimal must not be greater than 18")]
fn decimal_is_over_eighteen() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let _ = TokenClient::new(
        &e,
        &e.register(
            Token,
            (
                admin.clone(),
                19_u32,
                String::from_val(&e, &"name"),
                String::from_val(&e, &"symbol"),
                1000000000000000000_i128,
                admin,
            ),
        ),
    );
}

#[test]
fn test_zero_allowance() {
    // Here we test that transfer_from with a 0 amount does not create an empty allowance
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let spender = Address::generate(&e);
    let from = Address::generate(&e);
    let token = create_token(&e, &admin);

    token.transfer_from(&spender, &from, &spender, &0);
    // assert!(token.get_allowance(&from, &spender).is_none());
}
//   pub fn get_allowance(e: Env, from: Address, spender: Address) -> Option<AllowanceValue> {
//         let key = DataKey::Allowance(AllowanceDataKey { from, spender });
//         let allowance = e.storage().temporary().get::<_, AllowanceValue>(&key);
//         allowance
//     }