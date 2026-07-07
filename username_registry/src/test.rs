#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

fn setup() -> (Env, UsernameRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(UsernameRegistry, ());
    let client = UsernameRegistryClient::new(&env, &contract_id);
    (env, client)
}

#[test]
fn test_register_and_get() {
    let (env, client) = setup();
    let user1 = Address::generate(&env);
    let username = String::from_str(&env, "alice_123");

    assert!(client.is_available(&username));
    
    client.register_username(&user1, &username);
    
    assert!(!client.is_available(&username));
    
    let addr = client.get_address(&username);
    assert_eq!(addr, user1);
    
    let u = client.get_username(&user1);
    assert_eq!(u, username);
}

#[test]
#[should_panic]
fn test_invalid_username_length() {
    let (env, client) = setup();
    let user1 = Address::generate(&env);
    // Too short
    let username = String::from_str(&env, "ab");
    client.register_username(&user1, &username);
}

#[test]
#[should_panic]
fn test_invalid_username_chars() {
    let (env, client) = setup();
    let user1 = Address::generate(&env);
    // Invalid char @
    let username = String::from_str(&env, "alice@123");
    client.register_username(&user1, &username);
}

#[test]
fn test_update_username() {
    let (env, client) = setup();
    let user1 = Address::generate(&env);
    
    let old_username = String::from_str(&env, "alice_old");
    let new_username = String::from_str(&env, "alice_new");

    client.register_username(&user1, &old_username);
    assert_eq!(client.get_username(&user1), old_username);
    assert!(!client.is_available(&old_username));

    client.update_username(&user1, &new_username);
    
    assert_eq!(client.get_username(&user1), new_username);
    assert!(client.is_available(&old_username)); // Old one should be freed
    assert!(!client.is_available(&new_username));
}

#[test]
#[should_panic]
fn test_register_taken() {
    let (env, client) = setup();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let username = String::from_str(&env, "bob");

    client.register_username(&user1, &username);
    client.register_username(&user2, &username); // should panic
}
