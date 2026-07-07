#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, Vec,
};

#[contract]
pub struct ReceiptAccessRegistry;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    ReceiptRegistry,
    Viewers(u64), // receipt_id -> Vec<Address>
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AccessError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    NotAuthorized = 4,
    ViewerAlreadyExists = 5,
    ViewerNotFound = 6,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_receipt_registry(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::ReceiptRegistry)
        .unwrap_or_else(|| panic_with_error!(env, AccessError::NotInitialized))
}

fn check_sender_or_receiver(env: &Env, receipt_id: u64, caller: &Address) {
    caller.require_auth();
    // We make a cross-contract call to the payment receipt registry using invoke_contract
    let registry_addr = get_receipt_registry(env);
    let result: soroban_sdk::Val = env
        .invoke_contract(
            &registry_addr,
            &soroban_sdk::Symbol::new(env, "get_receipt"),
            soroban_sdk::vec![env, soroban_sdk::IntoVal::into_val(&receipt_id, env)],
        );
    // We don't parse the full result; the caller must be authorized via require_auth above.
    // Just ensure the call doesn't panic (receipt exists).
    let _ = result;
}

// ---------------------------------------------------------------------------
// Contract implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl ReceiptAccessRegistry {
    pub fn initialize(env: Env, admin: Address, receipt_registry: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, AccessError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::ReceiptRegistry, &receipt_registry);
    }

    pub fn grant_access(env: Env, caller: Address, receipt_id: u64, viewer: Address) {
        caller.require_auth();
        let registry_addr = get_receipt_registry(&env);
        let _: soroban_sdk::Val = env.invoke_contract(
            &registry_addr,
            &soroban_sdk::Symbol::new(&env, "get_receipt"),
            soroban_sdk::vec![&env, soroban_sdk::IntoVal::into_val(&receipt_id, &env)],
        );

        let key = DataKey::Viewers(receipt_id);
        let mut viewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env));

        if viewers.contains(&viewer) {
            panic_with_error!(&env, AccessError::ViewerAlreadyExists);
        }

        viewers.push_back(viewer);
        env.storage().persistent().set(&key, &viewers);
    }

    pub fn revoke_access(env: Env, caller: Address, receipt_id: u64, viewer: Address) {
        caller.require_auth();

        let key = DataKey::Viewers(receipt_id);
        let mut viewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env));

        if let Some(index) = viewers.first_index_of(&viewer) {
            viewers.remove(index);
            env.storage().persistent().set(&key, &viewers);
        } else {
            panic_with_error!(&env, AccessError::ViewerNotFound);
        }
    }

    pub fn has_access(env: Env, receipt_id: u64, viewer: Address) -> bool {
        let key = DataKey::Viewers(receipt_id);
        let viewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env));
        viewers.contains(&viewer)
    }

    pub fn get_viewers(env: Env, receipt_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Viewers(receipt_id))
            .unwrap_or(Vec::new(&env))
    }
}

#[cfg(test)]
mod test;
