#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Symbol,
};

#[contract]
pub struct ReserveVault;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Engine,
    TotalReserve(Address),
    LockedReserve(Address),
    PositionLock(u64),
    PositionToken(u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ReserveError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotEngine = 3,
    InvalidAmount = 4,
    InsufficientAvailableReserve = 5,
    InsufficientLockedReserve = 6,
    TokenMismatch = 7,
    AccountingBalanceTooLow = 8,
}

fn configured_engine(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Engine)
        .unwrap_or_else(|| panic_with_error!(env, ReserveError::NotInitialized))
}

fn require_engine(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != configured_engine(env) {
        panic_with_error!(env, ReserveError::NotEngine);
    }
}

fn read_i128(env: &Env, key: &DataKey) -> i128 {
    env.storage().persistent().get(key).unwrap_or(0)
}

#[contractimpl]
impl ReserveVault {
    pub fn initialize(env: Env, admin: Address, engine: Address) {
        if env.storage().instance().has(&DataKey::Engine) {
            panic_with_error!(&env, ReserveError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Engine, &engine);
    }

    pub fn deposit_reserve(env: Env, funder: Address, token: Address, amount: i128) {
        funder.require_auth();
        if amount <= 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }

        token::Client::new(&env, &token).transfer(
            &funder,
            &env.current_contract_address(),
            &amount,
        );
        let key = DataKey::TotalReserve(token.clone());
        let next_total = read_i128(&env, &key) + amount;
        let actual_balance =
            token::Client::new(&env, &token).balance(&env.current_contract_address());
        if actual_balance < next_total {
            panic_with_error!(&env, ReserveError::AccountingBalanceTooLow);
        }
        env.storage().persistent().set(&key, &next_total);
        env.events().publish(
            (Symbol::new(&env, "reserve_deposited"), funder),
            (token, amount),
        );
    }

    pub fn lock_payout_capacity(
        env: Env,
        engine: Address,
        token: Address,
        position_id: u64,
        amount: i128,
    ) {
        require_engine(&env, &engine);
        if amount <= 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }
        let available = Self::get_available_reserve(env.clone(), token.clone());
        if available < amount {
            panic_with_error!(&env, ReserveError::InsufficientAvailableReserve);
        }

        let locked_key = DataKey::LockedReserve(token.clone());
        env.storage()
            .persistent()
            .set(&locked_key, &(read_i128(&env, &locked_key) + amount));
        env.storage()
            .persistent()
            .set(&DataKey::PositionLock(position_id), &amount);
        env.storage()
            .persistent()
            .set(&DataKey::PositionToken(position_id), &token);
        env.events().publish(
            (Symbol::new(&env, "payout_capacity_locked"), position_id),
            amount,
        );
    }

    pub fn record_premium_deposit(
        env: Env,
        engine: Address,
        funder: Address,
        token: Address,
        amount: i128,
    ) {
        require_engine(&env, &engine);
        if amount <= 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }

        let key = DataKey::TotalReserve(token.clone());
        env.storage()
            .persistent()
            .set(&key, &(read_i128(&env, &key) + amount));
        env.events().publish(
            (Symbol::new(&env, "reserve_deposited"), funder),
            (token, amount),
        );
    }

    pub fn release_payout_capacity(env: Env, engine: Address, token: Address, position_id: u64) {
        require_engine(&env, &engine);
        let stored_token: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PositionToken(position_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReserveError::InsufficientLockedReserve));
        if stored_token != token {
            panic_with_error!(&env, ReserveError::TokenMismatch);
        }
        let locked_for_position = read_i128(&env, &DataKey::PositionLock(position_id));
        if locked_for_position <= 0 {
            return;
        }

        let locked_key = DataKey::LockedReserve(token);
        env.storage().persistent().set(
            &locked_key,
            &(read_i128(&env, &locked_key) - locked_for_position),
        );
        env.storage()
            .persistent()
            .set(&DataKey::PositionLock(position_id), &0_i128);
        env.events().publish(
            (Symbol::new(&env, "payout_capacity_released"), position_id),
            locked_for_position,
        );
    }

    pub fn pay_claim(
        env: Env,
        engine: Address,
        token: Address,
        user: Address,
        position_id: u64,
        payout_amount: i128,
    ) {
        require_engine(&env, &engine);
        if payout_amount <= 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }
        let locked_for_position = read_i128(&env, &DataKey::PositionLock(position_id));
        if locked_for_position < payout_amount {
            panic_with_error!(&env, ReserveError::InsufficientLockedReserve);
        }
        let stored_token: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PositionToken(position_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReserveError::InsufficientLockedReserve));
        if stored_token != token {
            panic_with_error!(&env, ReserveError::TokenMismatch);
        }

        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &user,
            &payout_amount,
        );
        let total_key = DataKey::TotalReserve(token.clone());
        let locked_key = DataKey::LockedReserve(token);
        env.storage()
            .persistent()
            .set(&total_key, &(read_i128(&env, &total_key) - payout_amount));
        env.storage().persistent().set(
            &locked_key,
            &(read_i128(&env, &locked_key) - locked_for_position),
        );
        env.storage()
            .persistent()
            .set(&DataKey::PositionLock(position_id), &0_i128);
        env.events().publish(
            (Symbol::new(&env, "claim_paid"), position_id),
            (user, payout_amount),
        );
    }

    pub fn get_total_reserve(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::TotalReserve(token))
    }

    pub fn get_locked_reserve(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::LockedReserve(token))
    }

    pub fn get_available_reserve(env: Env, token: Address) -> i128 {
        Self::get_total_reserve(env.clone(), token.clone()) - Self::get_locked_reserve(env, token)
    }

    pub fn get_locked_for_position(env: Env, position_id: u64) -> i128 {
        read_i128(&env, &DataKey::PositionLock(position_id))
    }
}
