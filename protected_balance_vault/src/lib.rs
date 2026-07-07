#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Symbol,
};

#[contract]
pub struct ProtectedBalanceVault;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Engine,
    PositionBalance(u64),
    TokenBalance(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ProtectedBalanceError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotEngine = 3,
    InvalidAmount = 4,
    InsufficientBalance = 5,
}

fn configured_engine(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Engine)
        .unwrap_or_else(|| panic_with_error!(env, ProtectedBalanceError::NotInitialized))
}

fn require_engine(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != configured_engine(env) {
        panic_with_error!(env, ProtectedBalanceError::NotEngine);
    }
}

fn read_i128(env: &Env, key: &DataKey) -> i128 {
    env.storage().persistent().get(key).unwrap_or(0)
}

#[contractimpl]
impl ProtectedBalanceVault {
    pub fn initialize(env: Env, admin: Address, engine: Address) {
        if env.storage().instance().has(&DataKey::Engine) {
            panic_with_error!(&env, ProtectedBalanceError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Engine, &engine);
    }

    pub fn deposit_from(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        amount: i128,
        position_id: u64,
    ) {
        require_engine(&env, &engine);
        if amount <= 0 {
            panic_with_error!(&env, ProtectedBalanceError::InvalidAmount);
        }

        token::Client::new(&env, &token).transfer(&user, &env.current_contract_address(), &amount);

        let position_key = DataKey::PositionBalance(position_id);
        let token_key = DataKey::TokenBalance(token.clone());
        env.storage()
            .persistent()
            .set(&position_key, &(read_i128(&env, &position_key) + amount));
        env.storage()
            .persistent()
            .set(&token_key, &(read_i128(&env, &token_key) + amount));
        env.events().publish(
            (Symbol::new(&env, "principal_deposited"), position_id),
            (user, token, amount),
        );
    }

    pub fn withdraw_to(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        amount: i128,
        position_id: u64,
    ) {
        require_engine(&env, &engine);
        if amount <= 0 {
            panic_with_error!(&env, ProtectedBalanceError::InvalidAmount);
        }

        let position_key = DataKey::PositionBalance(position_id);
        let token_key = DataKey::TokenBalance(token.clone());
        let position_balance = read_i128(&env, &position_key);
        if position_balance < amount {
            panic_with_error!(&env, ProtectedBalanceError::InsufficientBalance);
        }

        token::Client::new(&env, &token).transfer(&env.current_contract_address(), &user, &amount);
        env.storage()
            .persistent()
            .set(&position_key, &(position_balance - amount));
        env.storage()
            .persistent()
            .set(&token_key, &(read_i128(&env, &token_key) - amount));
        env.events().publish(
            (Symbol::new(&env, "principal_withdrawn"), position_id),
            (user, token, amount),
        );
    }

    pub fn get_position_balance(env: Env, position_id: u64) -> i128 {
        read_i128(&env, &DataKey::PositionBalance(position_id))
    }

    pub fn get_total_token_balance(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::TokenBalance(token))
    }
}
