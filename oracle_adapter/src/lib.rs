#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, Symbol,
};

#[contract]
pub struct OracleAdapter;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Price(Address),
    LastUpdated(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OracleError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    InvalidPrice = 4,
    MissingPrice = 5,
}

fn admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, OracleError::NotInitialized))
}

fn require_admin(env: &Env, candidate: &Address) {
    candidate.require_auth();
    if *candidate != admin(env) {
        panic_with_error!(env, OracleError::NotAdmin);
    }
}

#[contractimpl]
impl OracleAdapter {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, OracleError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn set_price(env: Env, admin: Address, asset: Address, price: i128) {
        require_admin(&env, &admin);
        if price <= 0 {
            panic_with_error!(&env, OracleError::InvalidPrice);
        }

        let timestamp = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::Price(asset.clone()), &price);
        env.storage()
            .persistent()
            .set(&DataKey::LastUpdated(asset.clone()), &timestamp);
        env.events().publish(
            (Symbol::new(&env, "price_updated"), asset),
            (price, timestamp),
        );
    }

    pub fn get_price(env: Env, asset: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Price(asset))
            .unwrap_or_else(|| panic_with_error!(&env, OracleError::MissingPrice))
    }

    pub fn get_last_updated(env: Env, asset: Address) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::LastUpdated(asset))
            .unwrap_or(0)
    }
}
