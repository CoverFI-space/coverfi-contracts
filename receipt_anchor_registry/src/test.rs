#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

fn setup() -> (Env, ReceiptAnchorRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ReceiptAnchorRegistry, ());
    let client = ReceiptAnchorRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn test_anchor_and_verify() {
    let (env, client, admin) = setup();
    
    let merkle_root = BytesN::from_array(&env, &[5; 32]);
    let count = 1000;

    let batch_id = client.anchor_batch(&admin, &merkle_root, &count);
    assert_eq!(batch_id, 1);

    let batch = client.get_batch(&batch_id);
    assert_eq!(batch.batch_id, 1);
    assert_eq!(batch.merkle_root, merkle_root);
    assert_eq!(batch.receipt_count, 1000);

    assert!(client.verify_batch(&batch_id, &merkle_root));
    
    let wrong_root = BytesN::from_array(&env, &[6; 32]);
    assert!(!client.verify_batch(&batch_id, &wrong_root));
}

#[test]
#[should_panic]
fn test_unauthorized_anchor() {
    let (env, client, _admin) = setup();
    let random_guy = Address::generate(&env);
    let merkle_root = BytesN::from_array(&env, &[5; 32]);
    
    client.anchor_batch(&random_guy, &merkle_root, &100);
}
