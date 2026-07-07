#![cfg(test)]

use super::*;
use payment_receipt_registry::{PaymentReceiptRegistry, PaymentReceiptRegistryClient};
use soroban_sdk::{testutils::Address as _, BytesN, Env, String};

fn setup() -> (
    Env,
    ReceiptAccessRegistryClient<'static>,
    PaymentReceiptRegistryClient<'static>,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy Receipt Registry
    let rr_id = env.register(PaymentReceiptRegistry, ());
    let rr_client = PaymentReceiptRegistryClient::new(&env, &rr_id);
    let admin = Address::generate(&env);
    rr_client.initialize(&admin);

    // Deploy Access Registry
    let ar_id = env.register(ReceiptAccessRegistry, ());
    let ar_client = ReceiptAccessRegistryClient::new(&env, &ar_id);
    ar_client.initialize(&admin, &rr_id);

    (env, ar_client, rr_client, admin)
}

#[test]
fn test_grant_and_has_access() {
    let (env, ar_client, rr_client, _admin) = setup();

    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let viewer1 = Address::generate(&env);

    let tx_hash = String::from_str(&env, "tx");
    let r_hash = BytesN::from_array(&env, &[0; 32]);
    let uri = String::from_str(&env, "uri");

    let receipt_id = rr_client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);

    // Sender and receiver should have access natively
    assert!(ar_client.has_access(&receipt_id, &sender));
    assert!(ar_client.has_access(&receipt_id, &receiver));
    assert!(!ar_client.has_access(&receipt_id, &viewer1));

    // Sender grants access
    ar_client.grant_access(&sender, &receipt_id, &viewer1);
    assert!(ar_client.has_access(&receipt_id, &viewer1));

    // Viewers list
    let viewers = ar_client.get_viewers(&receipt_id);
    assert_eq!(viewers.len(), 1);
    assert_eq!(viewers.get(0).unwrap(), viewer1);

    // Revoke access
    ar_client.revoke_access(&sender, &receipt_id, &viewer1);
    assert!(!ar_client.has_access(&receipt_id, &viewer1));
}

#[test]
#[should_panic]
fn test_unauthorized_grant() {
    let (env, ar_client, rr_client, _admin) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let random_guy = Address::generate(&env);
    let viewer = Address::generate(&env);

    let id = rr_client.create_receipt(
        &sender,
        &receiver,
        &String::from_str(&env, "t"),
        &BytesN::from_array(&env, &[0; 32]),
        &String::from_str(&env, "u"),
    );

    // Random guy tries to grant access
    ar_client.grant_access(&random_guy, &id, &viewer);
}
