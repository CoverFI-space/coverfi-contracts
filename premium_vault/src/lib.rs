#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Symbol,
};

#[contract]
pub struct PremiumVault;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Engine,
    ReserveVault,
    TotalPremiums(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PremiumError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotEngine = 3,
    InvalidAmount = 4,
    InsufficientPremiumBalance = 5,
}

fn configured_engine(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Engine)
        .unwrap_or_else(|| panic_with_error!(env, PremiumError::NotInitialized))
}

fn reserve_vault(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::ReserveVault)
        .unwrap_or_else(|| panic_with_error!(env, PremiumError::NotInitialized))
}

fn require_engine(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != configured_engine(env) {
        panic_with_error!(env, PremiumError::NotEngine);
    }
}

fn read_i128(env: &Env, key: &DataKey) -> i128 {
    env.storage().persistent().get(key).unwrap_or(0)
}

#[contractimpl]
impl PremiumVault {
    pub fn initialize(env: Env, admin: Address, engine: Address, reserve_vault: Address) {
        if env.storage().instance().has(&DataKey::Engine) {
            panic_with_error!(&env, PremiumError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Engine, &engine);
        env.storage()
            .instance()
            .set(&DataKey::ReserveVault, &reserve_vault);
    }

    pub fn collect_fee_from(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        amount: i128,
        position_id: u64,
    ) {
        require_engine(&env, &engine);
        if amount <= 0 {
            panic_with_error!(&env, PremiumError::InvalidAmount);
        }

        token::Client::new(&env, &token).transfer(&user, &env.current_contract_address(), &amount);
        let key = DataKey::TotalPremiums(token.clone());
        env.storage()
            .persistent()
            .set(&key, &(read_i128(&env, &key) + amount));
        env.events().publish(
            (Symbol::new(&env, "fee_collected"), position_id),
            (user, token, amount),
        );
    }

    pub fn forward_to_reserve(env: Env, token: Address, amount: i128) {
        configured_engine(&env).require_auth();
        if amount <= 0 {
            panic_with_error!(&env, PremiumError::InvalidAmount);
        }
        let balance = token::Client::new(&env, &token).balance(&env.current_contract_address());
        if balance < amount {
            panic_with_error!(&env, PremiumError::InsufficientPremiumBalance);
        }

        let reserve = reserve_vault(&env);
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &reserve,
            &amount,
        );
        env.events().publish(
            (Symbol::new(&env, "fee_forwarded_to_reserve"), token),
            amount,
        );
    }

    pub fn get_total_premiums(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::TotalPremiums(token))
    }
}
