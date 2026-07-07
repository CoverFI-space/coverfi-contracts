#![cfg(test)]

use super::*;
use payment_receipt_registry::{PaymentReceiptRegistry, PaymentReceiptRegistryClient};
use soroban_sdk::{testutils::Address as _, BytesN, Env, String};

fn setup() -> (
    Env,
    DisputeRegistryClient<'static>,
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

    // Deploy Dispute Registry
    let dr_id = env.register(DisputeRegistry, ());
    let dr_client = DisputeRegistryClient::new(&env, &dr_id);
    dr_client.initialize(&admin, &rr_id);

    (env, dr_client, rr_client, admin)
}

#[test]
fn test_dispute_flow() {
    let (env, dr_client, rr_client, admin) = setup();

    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);

    let tx_hash = String::from_str(&env, "tx");
    let r_hash = BytesN::from_array(&env, &[0; 32]);
    let uri = String::from_str(&env, "uri");

    let receipt_id = rr_client.create_receipt(&sender, &receiver, &tx_hash, &r_hash, &uri);

    // Open dispute
    let reason_hash = BytesN::from_array(&env, &[1; 32]);
    let dispute_id = dr_client.open_dispute(&sender, &receipt_id, &reason_hash);

    let d1 = dr_client.get_dispute(&dispute_id);
    assert_eq!(d1.status, DisputeStatus::Open);
    assert_eq!(d1.opener, sender);

    // Respond dispute
    let response_hash = BytesN::from_array(&env, &[2; 32]);
    dr_client.respond_dispute(&receiver, &dispute_id, &response_hash);

    let d2 = dr_client.get_dispute(&dispute_id);
    assert_eq!(d2.status, DisputeStatus::Responded);

    // Resolve dispute
    let result = String::from_str(&env, "Refund granted");
    dr_client.resolve_dispute(&admin, &dispute_id, &result);

    let d3 = dr_client.get_dispute(&dispute_id);
    assert_eq!(d3.status, DisputeStatus::Resolved);
    assert_eq!(d3.result.unwrap(), result);
}
