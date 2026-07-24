#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    crypto::bn254::{Bn254G1Affine, Bn254G2Affine},
    panic_with_error, Address, BytesN, Env, Symbol, Vec,
};

const INSTANCE_TTL_THRESHOLD: u32 = 30 * 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 120 * 17_280;

#[contract]
pub struct ZkVerifier;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZkConfig {
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitVerifier {
    pub circuit_id: BytesN<32>,
    pub verifier_hash: BytesN<32>,
    pub kind_hash: BytesN<32>,
    pub enabled: bool,
    pub registered_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofRecord {
    pub proof_id: u64,
    pub subject: Address,
    pub circuit_id: BytesN<32>,
    pub commitment: BytesN<32>,
    pub public_inputs_hash: BytesN<32>,
    pub proof_hash: BytesN<32>,
    pub verifier_hash: BytesN<32>,
    pub verified_at: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Config,
    NextProofId,
    Circuit(BytesN<32>),
    Proof(u64),
    // Only an explicitly trusted attestation path may set this key. A proof
    // record by itself is not evidence that a circuit was verified.
    VerifiedSubjectProof(Address, BytesN<32>),
    SubjectProof(Address, BytesN<32>),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ZkVerifierError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    CircuitNotRegistered = 4,
    CircuitDisabled = 5,
    VerifierMismatch = 6,
    ProofNotFound = 7,
    InvalidProof = 8,
}

fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

fn config(env: &Env) -> ZkConfig {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .unwrap_or_else(|| panic_with_error!(env, ZkVerifierError::NotInitialized))
}

fn require_admin(env: &Env, admin: &Address) {
    admin.require_auth();
    if *admin != config(env).admin {
        panic_with_error!(env, ZkVerifierError::NotAdmin);
    }
}

fn next_proof_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextProofId)
        .unwrap_or(1);
    env.storage()
        .instance()
        .set(&DataKey::NextProofId, &(id + 1));
    id
}

fn circuit(env: &Env, circuit_id: &BytesN<32>) -> CircuitVerifier {
    env.storage()
        .persistent()
        .get(&DataKey::Circuit(circuit_id.clone()))
        .unwrap_or_else(|| panic_with_error!(env, ZkVerifierError::CircuitNotRegistered))
}

fn require_enabled_verifier(
    env: &Env,
    circuit_id: &BytesN<32>,
    verifier_hash: &BytesN<32>,
) -> CircuitVerifier {
    let value = circuit(env, circuit_id);
    if !value.enabled {
        panic_with_error!(env, ZkVerifierError::CircuitDisabled);
    }
    if value.verifier_hash != *verifier_hash {
        panic_with_error!(env, ZkVerifierError::VerifierMismatch);
    }
    value
}

fn save_proof(
    env: &Env,
    subject: &Address,
    circuit_id: &BytesN<32>,
    commitment: &BytesN<32>,
    public_inputs_hash: &BytesN<32>,
    proof_hash: &BytesN<32>,
    verifier_hash: &BytesN<32>,
    verified: bool,
) -> u64 {
    require_enabled_verifier(env, circuit_id, verifier_hash);
    let proof_id = next_proof_id(env);
    let value = ProofRecord {
        proof_id,
        subject: subject.clone(),
        circuit_id: circuit_id.clone(),
        commitment: commitment.clone(),
        public_inputs_hash: public_inputs_hash.clone(),
        proof_hash: proof_hash.clone(),
        verifier_hash: verifier_hash.clone(),
        verified_at: env.ledger().timestamp(),
    };
    env.storage()
        .persistent()
        .set(&DataKey::Proof(proof_id), &value);
    env.storage().persistent().set(
        &DataKey::SubjectProof(subject.clone(), circuit_id.clone()),
        &proof_id,
    );
    if verified {
        env.storage().persistent().set(
            &DataKey::VerifiedSubjectProof(subject.clone(), circuit_id.clone()),
            &proof_id,
        );
    }
    env.events().publish(
        (
            Symbol::new(env, "zk_proof"),
            subject.clone(),
            circuit_id.clone(),
        ),
        proof_id,
    );
    proof_id
}

#[contractimpl]
impl ZkVerifier {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, ZkVerifierError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::Config, &ZkConfig { admin });
        env.storage().instance().set(&DataKey::NextProofId, &1_u64);
        extend_instance(&env);
    }

    pub fn get_config(env: Env) -> ZkConfig {
        config(&env)
    }

    pub fn register_verifier(
        env: Env,
        admin: Address,
        circuit_id: BytesN<32>,
        verifier_hash: BytesN<32>,
        kind_hash: BytesN<32>,
        enabled: bool,
    ) {
        require_admin(&env, &admin);
        let value = CircuitVerifier {
            circuit_id: circuit_id.clone(),
            verifier_hash,
            kind_hash,
            enabled,
            registered_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Circuit(circuit_id.clone()), &value);
        env.events()
            .publish((Symbol::new(&env, "zk_circuit"), circuit_id), value.enabled);
        extend_instance(&env);
    }

    pub fn set_verifier_enabled(env: Env, admin: Address, circuit_id: BytesN<32>, enabled: bool) {
        require_admin(&env, &admin);
        let mut value = circuit(&env, &circuit_id);
        value.enabled = enabled;
        env.storage()
            .persistent()
            .set(&DataKey::Circuit(circuit_id.clone()), &value);
        env.events()
            .publish((Symbol::new(&env, "zk_enabled"), circuit_id), enabled);
    }

    pub fn get_verifier(env: Env, circuit_id: BytesN<32>) -> CircuitVerifier {
        circuit(&env, &circuit_id)
    }

    pub fn record_proof(
        env: Env,
        subject: Address,
        circuit_id: BytesN<32>,
        commitment: BytesN<32>,
        public_inputs_hash: BytesN<32>,
        proof_hash: BytesN<32>,
        verifier_hash: BytesN<32>,
    ) -> u64 {
        subject.require_auth();
        save_proof(
            &env,
            &subject,
            &circuit_id,
            &commitment,
            &public_inputs_hash,
            &proof_hash,
            &verifier_hash,
            false,
        )
    }

    pub fn admin_record_proof(
        env: Env,
        admin: Address,
        subject: Address,
        circuit_id: BytesN<32>,
        commitment: BytesN<32>,
        public_inputs_hash: BytesN<32>,
        proof_hash: BytesN<32>,
        verifier_hash: BytesN<32>,
    ) -> u64 {
        require_admin(&env, &admin);
        save_proof(
            &env,
            &subject,
            &circuit_id,
            &commitment,
            &public_inputs_hash,
            &proof_hash,
            &verifier_hash,
            true,
        )
    }

    pub fn verify_bn254_pairing(
        env: Env,
        subject: Address,
        circuit_id: BytesN<32>,
        commitment: BytesN<32>,
        public_inputs_hash: BytesN<32>,
        proof_hash: BytesN<32>,
        verifier_hash: BytesN<32>,
        g1_points: Vec<Bn254G1Affine>,
        g2_points: Vec<Bn254G2Affine>,
    ) -> u64 {
        subject.require_auth();
        if !env.crypto().bn254().pairing_check(g1_points, g2_points) {
            panic_with_error!(&env, ZkVerifierError::InvalidProof);
        }
        save_proof(
            &env,
            &subject,
            &circuit_id,
            &commitment,
            &public_inputs_hash,
            &proof_hash,
            &verifier_hash,
            false,
        )
    }

    pub fn has_verified(env: Env, subject: Address, circuit_id: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::VerifiedSubjectProof(subject, circuit_id))
    }

    pub fn get_subject_proof_id(env: Env, subject: Address, circuit_id: BytesN<32>) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::VerifiedSubjectProof(subject, circuit_id))
    }

    pub fn get_proof(env: Env, proof_id: u64) -> ProofRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Proof(proof_id))
            .unwrap_or_else(|| panic_with_error!(&env, ZkVerifierError::ProofNotFound))
    }
}

#[cfg(test)]
mod test;
