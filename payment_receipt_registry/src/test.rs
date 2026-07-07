#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

fn setup() -> (Env, PaymentReceiptRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PaymentReceiptRegistry, ());
    let client = PaymentReceiptRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn dummy_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1u8; 32])
}

fn other_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[2u8; 32])
}

#[test]
fn test_create_and_get_receipt() {
    let (env, client, _admin) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let tx_hash = String::from_str(&env, "abc123tx");
    let r_hash = dummy_hash(&env);
    let uri = String::from_str(&env, "ipfs://encrypted");

    let id = client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);
    assert_eq!(id, 1);

    let r = client.get_receipt(&id);
    assert_eq!(r.receipt_id, 1);
    assert_eq!(r.sender, sender);
    assert_eq!(r.receiver, receiver);
    assert_eq!(r.receipt_hash, r_hash);
    assert_eq!(r.status, ReceiptStatus::Active);
}

#[test]
fn test_sent_and_received_lists() {
    let (env, client, _admin) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let tx_hash = String::from_str(&env, "tx1");
    let r_hash = dummy_hash(&env);
    let uri = String::from_str(&env, "uri1");

    let id1 = client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);
    let id2 = client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);

    let sent = client.get_sent_receipts(&sender);
    assert_eq!(sent.len(), 2);
    assert_eq!(sent.get(0).unwrap(), id1);
    assert_eq!(sent.get(1).unwrap(), id2);

    let received = client.get_received_receipts(&receiver);
    assert_eq!(received.len(), 2);
}

#[test]
fn test_verify_receipt() {
    let (env, client, _admin) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let tx_hash = String::from_str(&env, "txh");
    let r_hash = dummy_hash(&env);
    let uri = String::from_str(&env, "uri");

    let id = client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);

    assert!(client.verify_receipt(&id, &r_hash));
    assert!(!client.verify_receipt(&id, &other_hash(&env)));
}

#[test]
fn test_mark_disputed() {
    let (env, client, _admin) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let tx_hash = String::from_str(&env, "txh");
    let r_hash = dummy_hash(&env);
    let uri = String::from_str(&env, "uri");

    let id = client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);

    client.mark_disputed(&sender, &id);
    let r = client.get_receipt(&id);
    assert_eq!(r.status, ReceiptStatus::Disputed);
}

#[test]
#[should_panic]
fn test_double_initialize_panics() {
    let (env, client, admin) = setup();
    let _ = &env;
    client.initialize(&admin);
}

#[test]
#[should_panic]
fn test_mark_disputed_twice_panics() {
    let (env, client, _admin) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let tx_hash = String::from_str(&env, "txh");
    let r_hash = dummy_hash(&env);
    let uri = String::from_str(&env, "uri");

    let id = client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);
    client.mark_disputed(&sender, &id);
    client.mark_disputed(&sender, &id); // should panic
}
