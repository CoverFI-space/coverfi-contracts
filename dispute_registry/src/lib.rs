#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, vec, Address, BytesN,
    Env, IntoVal, String, Symbol, Val,
};

#[contract]
pub struct DisputeRegistry;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    Open,
    Responded,
    Resolved,
}

#[contracttype]
#[derive(Clone)]
pub struct Dispute {
    pub dispute_id: u64,
    pub receipt_id: u64,
    pub opener: Address,
    pub reason_hash: BytesN<32>,
    pub response_hash: Option<BytesN<32>>,
    pub result: Option<String>,
    pub status: DisputeStatus,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    ReceiptRegistry,
    NextDisputeId,
    Dispute(u64),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DisputeError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    DisputeNotFound = 4,
    NotAuthorized = 5,
    InvalidStatus = 6,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_receipt_registry(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::ReceiptRegistry)
        .unwrap_or_else(|| panic_with_error!(env, DisputeError::NotInitialized))
}

fn check_auth_and_verify_party(env: &Env, receipt_id: u64, caller: &Address) {
    caller.require_auth();
    let registry_addr = get_receipt_registry(env);
    // Cross-contract call without importing the client spec (avoids duplicate fn export)
    let _: Val = env.invoke_contract(
        &registry_addr,
        &Symbol::new(env, "get_receipt"),
        vec![env, receipt_id.into_val(env)],
    );
}

fn next_dispute_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextDisputeId)
        .unwrap_or(1_u64);
    env.storage()
        .instance()
        .set(&DataKey::NextDisputeId, &(id + 1));
    id
}

// ---------------------------------------------------------------------------
// Contract implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl DisputeRegistry {
    pub fn initialize(env: Env, admin: Address, receipt_registry: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, DisputeError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::ReceiptRegistry, &receipt_registry);
        env.storage()
            .instance()
            .set(&DataKey::NextDisputeId, &1_u64);
    }

    pub fn open_dispute(env: Env, user: Address, receipt_id: u64, reason_hash: BytesN<32>) -> u64 {
        check_auth_and_verify_party(&env, receipt_id, &user);

        // Mark receipt as disputed via cross-contract invoke
        let registry_addr = get_receipt_registry(&env);
        let _: Val = env.invoke_contract(
            &registry_addr,
            &Symbol::new(&env, "mark_disputed"),
            vec![&env, user.clone().into_val(&env), receipt_id.into_val(&env)],
        );

        let id = next_dispute_id(&env);
        let dispute = Dispute {
            dispute_id: id,
            receipt_id,
            opener: user,
            reason_hash,
            response_hash: None,
            result: None,
            status: DisputeStatus::Open,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Dispute(id), &dispute);

        id
    }

    pub fn respond_dispute(env: Env, user: Address, dispute_id: u64, response_hash: BytesN<32>) {
        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic_with_error!(&env, DisputeError::DisputeNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, DisputeError::InvalidStatus);
        }

        check_auth_and_verify_party(&env, dispute.receipt_id, &user);

        if dispute.opener == user {
            panic_with_error!(&env, DisputeError::NotAuthorized);
        }

        dispute.response_hash = Some(response_hash);
        dispute.status = DisputeStatus::Responded;

        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &dispute);
    }

    pub fn resolve_dispute(env: Env, admin: Address, dispute_id: u64, result: String) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic_with_error!(&env, DisputeError::NotAdmin);
        }

        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic_with_error!(&env, DisputeError::DisputeNotFound));

        if dispute.status == DisputeStatus::Resolved {
            panic_with_error!(&env, DisputeError::InvalidStatus);
        }

        dispute.result = Some(result);
        dispute.status = DisputeStatus::Resolved;

        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &dispute);
    }

    pub fn get_dispute(env: Env, dispute_id: u64) -> Dispute {
        env.storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic_with_error!(&env, DisputeError::DisputeNotFound))
    }
}

#[cfg(test)]
mod test;
