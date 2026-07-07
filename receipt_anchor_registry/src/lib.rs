#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env,
};

#[contract]
pub struct ReceiptAnchorRegistry;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Batch {
    pub batch_id: u64,
    pub merkle_root: BytesN<32>,
    pub receipt_count: u32,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    NextBatchId,
    Batch(u64),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AnchorError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    BatchNotFound = 4,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn next_batch_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextBatchId)
        .unwrap_or(1_u64);
    env.storage()
        .instance()
        .set(&DataKey::NextBatchId, &(id + 1));
    id
}

// ---------------------------------------------------------------------------
// Contract implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl ReceiptAnchorRegistry {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, AnchorError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextBatchId, &1_u64);
    }

    pub fn anchor_batch(env: Env, admin: Address, merkle_root: BytesN<32>, receipt_count: u32) -> u64 {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic_with_error!(&env, AnchorError::NotAdmin);
        }

        let id = next_batch_id(&env);
        let batch = Batch {
            batch_id: id,
            merkle_root,
            receipt_count,
            created_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Batch(id), &batch);

        id
    }

    pub fn verify_batch(env: Env, batch_id: u64, merkle_root: BytesN<32>) -> bool {
        let batch: Batch = env
            .storage()
            .persistent()
            .get(&DataKey::Batch(batch_id))
            .unwrap_or_else(|| panic_with_error!(&env, AnchorError::BatchNotFound));

        batch.merkle_root == merkle_root
    }

    pub fn get_batch(env: Env, batch_id: u64) -> Batch {
        env.storage()
            .persistent()
            .get(&DataKey::Batch(batch_id))
            .unwrap_or_else(|| panic_with_error!(&env, AnchorError::BatchNotFound))
    }
}

#[cfg(test)]
mod test;
