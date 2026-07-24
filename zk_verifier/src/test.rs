extern crate std;

use super::{ZkVerifier, ZkVerifierClient};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

struct Fixture {
    env: Env,
    client: ZkVerifierClient<'static>,
    admin: Address,
    user: Address,
    circuit_id: BytesN<32>,
    verifier_hash: BytesN<32>,
    kind_hash: BytesN<32>,
}

fn bytes(env: &Env, value: u8) -> BytesN<32> {
    BytesN::from_array(env, &[value; 32])
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let id = env.register(ZkVerifier, ());
    let client = ZkVerifierClient::new(&env, &id);
    client.initialize(&admin);
    let circuit_id = bytes(&env, 1);
    let verifier_hash = bytes(&env, 2);
    let kind_hash = bytes(&env, 3);
    client.register_verifier(&admin, &circuit_id, &verifier_hash, &kind_hash, &true);

    Fixture {
        env,
        client,
        admin,
        user,
        circuit_id,
        verifier_hash,
        kind_hash,
    }
}

#[test]
fn subject_record_is_not_a_verified_proof() {
    let fixture = setup();
    assert!(!fixture
        .client
        .has_verified(&fixture.user, &fixture.circuit_id));

    let proof_id = fixture.client.record_proof(
        &fixture.user,
        &fixture.circuit_id,
        &bytes(&fixture.env, 10),
        &bytes(&fixture.env, 11),
        &bytes(&fixture.env, 12),
        &fixture.verifier_hash,
    );

    assert_eq!(proof_id, 1);
    assert!(!fixture
        .client
        .has_verified(&fixture.user, &fixture.circuit_id));
    assert_eq!(
        fixture
            .client
            .get_subject_proof_id(&fixture.user, &fixture.circuit_id),
        None
    );
    let proof = fixture.client.get_proof(&proof_id);
    assert_eq!(proof.subject, fixture.user);
    assert_eq!(proof.proof_hash, bytes(&fixture.env, 12));
}

#[test]
fn admin_can_anchor_backend_verified_proof() {
    let fixture = setup();
    let proof_id = fixture.client.admin_record_proof(
        &fixture.admin,
        &fixture.user,
        &fixture.circuit_id,
        &bytes(&fixture.env, 20),
        &bytes(&fixture.env, 21),
        &bytes(&fixture.env, 22),
        &fixture.verifier_hash,
    );
    assert_eq!(proof_id, 1);
    assert!(fixture
        .client
        .has_verified(&fixture.user, &fixture.circuit_id));
}

#[test]
fn admin_can_disable_verifier() {
    let fixture = setup();
    fixture
        .client
        .set_verifier_enabled(&fixture.admin, &fixture.circuit_id, &false);
    let verifier = fixture.client.get_verifier(&fixture.circuit_id);
    assert!(!verifier.enabled);
    assert_eq!(verifier.kind_hash, fixture.kind_hash);
}

#[test]
#[should_panic]
fn verifier_hash_must_match_registered_circuit() {
    let fixture = setup();
    fixture.client.record_proof(
        &fixture.user,
        &fixture.circuit_id,
        &bytes(&fixture.env, 10),
        &bytes(&fixture.env, 11),
        &bytes(&fixture.env, 12),
        &bytes(&fixture.env, 99),
    );
}
