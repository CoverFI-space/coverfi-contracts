#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Env,
};

struct Fixture {
    env: Env,
    client: UsernameRegistryClient<'static>,
    user: Address,
    other: Address,
    treasury: Address,
    automation: Address,
    token: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let other = Address::generate(&env);
    let treasury = Address::generate(&env);
    let automation = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&user, &10_000_000_000);
    token::StellarAssetClient::new(&env, &token).mint(&other, &10_000_000_000);
    let id = env.register(UsernameRegistry, ());
    let client = UsernameRegistryClient::new(&env, &id);
    client.initialize(&admin, &token, &treasury, &automation);
    Fixture {
        env,
        client,
        user,
        other,
        treasury,
        automation,
        token,
    }
}

#[test]
fn registration_charges_scarcity_price_and_routes_revenue() {
    let fixture = setup();
    let name = String::from_str(&fixture.env, "alice");
    let quote = fixture.client.quote_registration(&name, &1);
    assert_eq!(quote.total_fee, STANDARD_PRICE);
    let value = fixture.client.register_username(&fixture.user, &name, &1);
    assert_eq!(value.expires_at, 1_000 + LEASE_SECONDS);
    assert_eq!(fixture.client.get_address(&name), fixture.user);
    assert_eq!(fixture.client.get_username(&fixture.user), name);
    let token = token::Client::new(&fixture.env, &fixture.token);
    assert_eq!(token.balance(&fixture.treasury), 24_000_000);
    assert_eq!(token.balance(&fixture.automation), 6_000_000);
    assert_eq!(fixture.client.get_revenue(), (24_000_000, 6_000_000));
}

#[test]
fn short_names_have_higher_prices() {
    let fixture = setup();
    assert_eq!(
        fixture
            .client
            .quote_registration(&String::from_str(&fixture.env, "abc"), &1)
            .base_fee,
        THREE_CHAR_PRICE
    );
    assert_eq!(
        fixture
            .client
            .quote_registration(&String::from_str(&fixture.env, "abcd"), &1)
            .base_fee,
        FOUR_CHAR_PRICE
    );
}

#[test]
fn grace_period_blocks_takeover_but_allows_renewal() {
    let fixture = setup();
    let name = String::from_str(&fixture.env, "alice");
    fixture.client.register_username(&fixture.user, &name, &1);
    fixture
        .env
        .ledger()
        .set_timestamp(1_000 + LEASE_SECONDS + GRACE_SECONDS - 1);
    assert!(!fixture.client.is_available(&name));
    fixture.client.renew_username(&fixture.user, &1);
    assert!(!fixture.client.is_available(&name));
}

#[test]
fn public_reregistration_has_declining_premium() {
    let fixture = setup();
    let name = String::from_str(&fixture.env, "alice");
    fixture.client.register_username(&fixture.user, &name, &1);
    fixture
        .env
        .ledger()
        .set_timestamp(1_000 + LEASE_SECONDS + GRACE_SECONDS + 1);
    assert!(fixture.client.is_available(&name));
    let priced = fixture.client.quote_registration(&name, &1);
    assert!(priced.scarcity_premium > 0);
    fixture.client.register_username(&fixture.other, &name, &1);
    assert_eq!(fixture.client.get_address(&name), fixture.other);
}

#[test]
fn paid_rename_releases_old_name() {
    let fixture = setup();
    let old = String::from_str(&fixture.env, "alice_old");
    let new = String::from_str(&fixture.env, "alice_new");
    fixture.client.register_username(&fixture.user, &old, &1);
    fixture.client.change_username(&fixture.user, &new, &1);
    assert!(fixture.client.is_available(&old));
    assert_eq!(fixture.client.get_username(&fixture.user), new);
}

#[test]
#[should_panic]
fn reserved_name_is_rejected() {
    let fixture = setup();
    fixture.client.register_username(
        &fixture.user,
        &String::from_str(&fixture.env, "support"),
        &1,
    );
}

#[test]
#[should_panic]
fn wallet_cannot_hold_two_live_names() {
    let fixture = setup();
    fixture.client.register_username(
        &fixture.user,
        &String::from_str(&fixture.env, "alice_one"),
        &1,
    );
    fixture.client.register_username(
        &fixture.user,
        &String::from_str(&fixture.env, "alice_two"),
        &1,
    );
}

#[test]
#[should_panic]
fn uppercase_is_rejected() {
    let fixture = setup();
    fixture
        .client
        .register_username(&fixture.user, &String::from_str(&fixture.env, "Alice"), &1);
}
