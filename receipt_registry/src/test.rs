extern crate std;

use super::{DisputeStatus, ReceiptRegistry, ReceiptRegistryClient};
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env};

struct Fixture {
    env: Env,
    client: ReceiptRegistryClient<'static>,
    admin: Address,
    sender: Address,
    viewer: Address,
    treasury: Address,
    automation: Address,
    token: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let viewer = Address::generate(&env);
    let treasury = Address::generate(&env);
    let automation = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&sender, &1_000_000_000);
    let id = env.register(ReceiptRegistry, ());
    let client = ReceiptRegistryClient::new(&env, &id);
    client.initialize(&admin, &token, &treasury, &automation);
    Fixture {
        env,
        client,
        admin,
        sender,
        viewer,
        treasury,
        automation,
        token,
    }
}

fn create_receipt(fixture: &Fixture) -> u64 {
    fixture.client.create_receipt(
        &fixture.sender,
        &BytesN::from_array(&fixture.env, &[1; 32]),
        &BytesN::from_array(&fixture.env, &[11; 32]),
    )
}

#[test]
fn receipt_fee_routes_without_touching_payment_value() {
    let fixture = setup();
    let id = create_receipt(&fixture);
    assert_eq!(id, 1);
    let token = token::Client::new(&fixture.env, &fixture.token);
    assert_eq!(token.balance(&fixture.sender), 999_000_000);
    assert_eq!(token.balance(&fixture.treasury), 1_000_000);
    assert_eq!(token.balance(&fixture.automation), 0);
    assert!(fixture
        .client
        .verify_receipt(&id, &BytesN::from_array(&fixture.env, &[1; 32])));
}

#[test]
fn merchant_batch_uses_minimum_and_per_receipt_pricing() {
    let fixture = setup();
    assert_eq!(fixture.client.quote_batch_fee(&10), 10_000_000);
    assert_eq!(fixture.client.quote_batch_fee(&1_000), 20_000_000);
    let id = fixture.client.anchor_batch(
        &fixture.sender,
        &BytesN::from_array(&fixture.env, &[2; 32]),
        &1_000,
    );
    assert_eq!(fixture.client.get_batch(&id).fee_paid, 20_000_000);
}

#[test]
fn dispute_bond_is_held_then_refunded() {
    let fixture = setup();
    let receipt_id = create_receipt(&fixture);
    let token = token::Client::new(&fixture.env, &fixture.token);
    let before = token.balance(&fixture.sender);
    let dispute_id = fixture.client.open_dispute(
        &fixture.sender,
        &receipt_id,
        &BytesN::from_array(&fixture.env, &[3; 32]),
    );
    assert_eq!(token.balance(&fixture.sender), before - 10_000_000);
    fixture.client.resolve_dispute(
        &fixture.admin,
        &dispute_id,
        &BytesN::from_array(&fixture.env, &[5; 32]),
    );
    let dispute = fixture.client.get_dispute(&dispute_id);
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    assert!(dispute.bond_refunded);
    assert_eq!(token.balance(&fixture.sender), before);
}

#[test]
fn encrypted_reference_access_can_be_granted_and_revoked() {
    let fixture = setup();
    let receipt_id = create_receipt(&fixture);
    assert!(!fixture.client.has_access(&receipt_id, &fixture.viewer));
    fixture
        .client
        .grant_access(&fixture.sender, &receipt_id, &fixture.viewer);
    assert!(fixture.client.has_access(&receipt_id, &fixture.viewer));
    fixture
        .client
        .revoke_access(&fixture.sender, &receipt_id, &fixture.viewer);
    assert!(!fixture.client.has_access(&receipt_id, &fixture.viewer));
}

#[test]
fn consent_anchors_only_wallet_hash_and_timestamp() {
    let fixture = setup();
    let hash = BytesN::from_array(&fixture.env, &[9; 32]);
    fixture
        .client
        .record_terms_acceptance(&fixture.sender, &hash);
    let value = fixture.client.get_terms_acceptance(&fixture.sender);
    assert_eq!(value.user, fixture.sender);
    assert_eq!(value.terms_hash, hash);
}

#[test]
#[should_panic]
fn duplicate_payment_hash_is_rejected() {
    let fixture = setup();
    create_receipt(&fixture);
    create_receipt(&fixture);
}
