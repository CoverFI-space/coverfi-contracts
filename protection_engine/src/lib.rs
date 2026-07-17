#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, IntoVal,
    Symbol, Val, Vec,
};

const BPS_DENOMINATOR: i128 = 10_000;
const MIN_FEE_BPS: u32 = 20;
const MAX_FEE_BPS: u32 = 250;
const MIN_MAX_PAYOUT_BPS: u32 = 100;
const MAX_MAX_PAYOUT_BPS: u32 = 5_000;
const ONE_DAY_SECONDS: u64 = 86_400;
const MAX_DURATION_SECONDS: u64 = 30 * ONE_DAY_SECONDS;
const ONE_DAY_FEE_BPS: u32 = 30;
const SEVEN_DAY_FEE_BPS: u32 = 100;
const FOURTEEN_DAY_FEE_BPS: u32 = 150;
const THIRTY_DAY_FEE_BPS: u32 = 250;

#[contract]
pub struct ProtectionEngine;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PositionStatus {
    Active,
    Triggered,
    Expired,
    Claimed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub id: u64,
    pub owner: Address,
    pub protected_asset: Address,
    pub payout_asset: Address,
    pub protected_amount: i128,
    pub fee_paid: i128,
    pub entry_price: i128,
    pub start_time: u64,
    pub expiry_time: u64,
    pub status: PositionStatus,
    pub claimable_payout: i128,
    pub principal_withdrawn: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub admin: Address,
    pub protected_balance_vault: Address,
    pub reserve_vault: Address,
    pub oracle_adapter: Address,
    pub fee_bps: u32,
    pub max_payout_bps: u32,
    pub max_oracle_age_seconds: u64,
    pub paused: bool,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Config,
    NextPositionId,
    Position(u64),
    UserPositions(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EngineError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    Paused = 4,
    InvalidFeeBps = 5,
    InvalidMaxPayoutBps = 6,
    InvalidAmount = 7,
    InvalidDuration = 8,
    InvalidEntryPrice = 9,
    PositionNotFound = 10,
    NotOwner = 11,
    InvalidStatus = 12,
    NotExpired = 13,
    AlreadyWithdrawn = 14,
    PriceNotBelowEntry = 15,
    TriggerWindowClosed = 16,
    NothingToClaim = 17,
    StaleOraclePrice = 18,
}

fn config(env: &Env) -> Config {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .unwrap_or_else(|| panic_with_error!(env, EngineError::NotInitialized))
}

fn set_config(env: &Env, config: &Config) {
    env.storage().instance().set(&DataKey::Config, config);
}

fn require_admin(env: &Env, admin: &Address) {
    admin.require_auth();
    if *admin != config(env).admin {
        panic_with_error!(env, EngineError::NotAdmin);
    }
}

fn require_live(env: &Env) -> Config {
    let cfg = config(env);
    if cfg.paused {
        panic_with_error!(env, EngineError::Paused);
    }
    cfg
}

fn validate_fee_bps(env: &Env, fee_bps: u32) {
    if !(MIN_FEE_BPS..=MAX_FEE_BPS).contains(&fee_bps) {
        panic_with_error!(env, EngineError::InvalidFeeBps);
    }
}

fn validate_max_payout_bps(env: &Env, max_payout_bps: u32) {
    if !(MIN_MAX_PAYOUT_BPS..=MAX_MAX_PAYOUT_BPS).contains(&max_payout_bps) {
        panic_with_error!(env, EngineError::InvalidMaxPayoutBps);
    }
}

fn position(env: &Env, position_id: u64) -> Position {
    env.storage()
        .persistent()
        .get(&DataKey::Position(position_id))
        .unwrap_or_else(|| panic_with_error!(env, EngineError::PositionNotFound))
}

fn set_position(env: &Env, position: &Position) {
    env.storage()
        .persistent()
        .set(&DataKey::Position(position.id), position);
}

fn next_position_id(env: &Env) -> u64 {
    let id = env
        .storage()
        .instance()
        .get(&DataKey::NextPositionId)
        .unwrap_or(1_u64);
    env.storage()
        .instance()
        .set(&DataKey::NextPositionId, &(id + 1));
    id
}

fn user_positions(env: &Env, user: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::UserPositions(user.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

fn save_user_position(env: &Env, user: &Address, position_id: u64) {
    let mut ids = user_positions(env, user);
    ids.push_back(position_id);
    env.storage()
        .persistent()
        .set(&DataKey::UserPositions(user.clone()), &ids);
}

fn amount_by_bps(amount: i128, bps: u32) -> i128 {
    amount * i128::from(bps) / BPS_DENOMINATOR
}

fn fee_bps_for_duration(env: &Env, duration_seconds: u64) -> u32 {
    if duration_seconds == 0 || duration_seconds > MAX_DURATION_SECONDS {
        panic_with_error!(env, EngineError::InvalidDuration);
    }

    if duration_seconds <= ONE_DAY_SECONDS {
        return ONE_DAY_FEE_BPS;
    }
    if duration_seconds <= 7 * ONE_DAY_SECONDS {
        return SEVEN_DAY_FEE_BPS;
    }
    if duration_seconds <= 14 * ONE_DAY_SECONDS {
        return FOURTEEN_DAY_FEE_BPS;
    }

    THIRTY_DAY_FEE_BPS
}

fn invoke_unit(env: &Env, contract: &Address, name: &str, args: Vec<Val>) {
    env.invoke_contract::<()>(contract, &Symbol::new(env, name), args);
}

fn invoke_i128(env: &Env, contract: &Address, name: &str, args: Vec<Val>) -> i128 {
    env.invoke_contract::<i128>(contract, &Symbol::new(env, name), args)
}

fn invoke_u64(env: &Env, contract: &Address, name: &str, args: Vec<Val>) -> u64 {
    env.invoke_contract::<u64>(contract, &Symbol::new(env, name), args)
}

fn fresh_oracle_price(env: &Env, cfg: &Config, asset: &Address) -> i128 {
    let price = invoke_i128(
        env,
        &cfg.oracle_adapter,
        "get_price",
        Vec::from_array(env, [asset.clone().into_val(env)]),
    );
    let last_updated = invoke_u64(
        env,
        &cfg.oracle_adapter,
        "get_last_updated",
        Vec::from_array(env, [asset.clone().into_val(env)]),
    );
    if last_updated == 0
        || cfg.max_oracle_age_seconds == 0
        || env.ledger().timestamp() > last_updated + cfg.max_oracle_age_seconds
    {
        panic_with_error!(env, EngineError::StaleOraclePrice);
    }
    if price <= 0 {
        panic_with_error!(env, EngineError::InvalidEntryPrice);
    }
    price
}

#[contractimpl]
impl ProtectionEngine {
    pub fn initialize(
        env: Env,
        admin: Address,
        protected_balance_vault: Address,
        reserve_vault: Address,
        oracle_adapter: Address,
        fee_bps: u32,
        max_payout_bps: u32,
        max_oracle_age_seconds: u64,
    ) {
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, EngineError::AlreadyInitialized);
        }
        admin.require_auth();
        validate_fee_bps(&env, fee_bps);
        validate_max_payout_bps(&env, max_payout_bps);

        set_config(
            &env,
            &Config {
                admin,
                protected_balance_vault,
                reserve_vault,
                oracle_adapter,
                fee_bps,
                max_payout_bps,
                max_oracle_age_seconds,
                paused: false,
            },
        );
        env.storage()
            .instance()
            .set(&DataKey::NextPositionId, &1_u64);
    }

    pub fn create_position(
        env: Env,
        user: Address,
        protected_asset: Address,
        payout_asset: Address,
        protected_amount: i128,
        duration_seconds: u64,
    ) -> u64 {
        let cfg = require_live(&env);
        user.require_auth();
        if protected_amount <= 0 {
            panic_with_error!(&env, EngineError::InvalidAmount);
        }
        if duration_seconds == 0 || duration_seconds > MAX_DURATION_SECONDS {
            panic_with_error!(&env, EngineError::InvalidDuration);
        }
        let entry_price = fresh_oracle_price(&env, &cfg, &protected_asset);

        let position_id = next_position_id(&env);
        let fee_paid =
            Self::get_fee_quote_for_duration(env.clone(), protected_amount, duration_seconds);
        let max_payout = Self::get_max_payout(env.clone(), protected_amount);
        let engine = env.current_contract_address();

        invoke_unit(
            &env,
            &cfg.protected_balance_vault,
            "deposit_from",
            Vec::from_array(
                &env,
                [
                    engine.clone().into_val(&env),
                    user.clone().into_val(&env),
                    protected_asset.clone().into_val(&env),
                    protected_amount.into_val(&env),
                    position_id.into_val(&env),
                ],
            ),
        );
        invoke_unit(
            &env,
            &cfg.reserve_vault,
            "collect_premium_from",
            Vec::from_array(
                &env,
                [
                    engine.clone().into_val(&env),
                    user.clone().into_val(&env),
                    protected_asset.clone().into_val(&env),
                    fee_paid.into_val(&env),
                    position_id.into_val(&env),
                ],
            ),
        );
        invoke_unit(
            &env,
            &cfg.reserve_vault,
            "lock_payout_capacity",
            Vec::from_array(
                &env,
                [
                    engine.into_val(&env),
                    payout_asset.clone().into_val(&env),
                    position_id.into_val(&env),
                    max_payout.into_val(&env),
                ],
            ),
        );

        let start_time = env.ledger().timestamp();
        let expiry_time = start_time + duration_seconds;
        let record = Position {
            id: position_id,
            owner: user.clone(),
            protected_asset: protected_asset.clone(),
            payout_asset,
            protected_amount,
            fee_paid,
            entry_price,
            start_time,
            expiry_time,
            status: PositionStatus::Active,
            claimable_payout: 0,
            principal_withdrawn: false,
        };
        set_position(&env, &record);
        save_user_position(&env, &user, position_id);
        env.events().publish(
            (Symbol::new(&env, "position_created"), position_id),
            (
                user,
                protected_asset,
                protected_amount,
                fee_paid,
                entry_price,
                expiry_time,
            ),
        );
        position_id
    }

    pub fn check_and_trigger(env: Env, position_id: u64) {
        let cfg = require_live(&env);
        let mut pos = position(&env, position_id);
        if pos.status != PositionStatus::Active {
            panic_with_error!(&env, EngineError::InvalidStatus);
        }
        if env.ledger().timestamp() > pos.expiry_time {
            panic_with_error!(&env, EngineError::TriggerWindowClosed);
        }

        let current_price = fresh_oracle_price(&env, &cfg, &pos.protected_asset);
        if current_price >= pos.entry_price {
            panic_with_error!(&env, EngineError::PriceNotBelowEntry);
        }

        let loss = pos.entry_price - current_price;
        let calculated = pos.protected_amount * loss / pos.entry_price;
        if calculated <= 0 {
            panic_with_error!(&env, EngineError::NothingToClaim);
        }
        let max_payout = Self::get_max_payout(env.clone(), pos.protected_amount);
        pos.claimable_payout = if calculated > max_payout {
            max_payout
        } else {
            calculated
        };
        pos.status = PositionStatus::Triggered;
        set_position(&env, &pos);
        env.events().publish(
            (Symbol::new(&env, "position_triggered"), position_id),
            (current_price, pos.claimable_payout),
        );
    }

    pub fn claim_payout(env: Env, user: Address, position_id: u64) {
        let cfg = require_live(&env);
        user.require_auth();
        let mut pos = position(&env, position_id);
        if pos.owner != user {
            panic_with_error!(&env, EngineError::NotOwner);
        }
        if pos.status != PositionStatus::Triggered {
            panic_with_error!(&env, EngineError::InvalidStatus);
        }
        if pos.claimable_payout <= 0 {
            panic_with_error!(&env, EngineError::NothingToClaim);
        }

        invoke_unit(
            &env,
            &cfg.reserve_vault,
            "pay_claim",
            Vec::from_array(
                &env,
                [
                    env.current_contract_address().into_val(&env),
                    pos.payout_asset.clone().into_val(&env),
                    user.clone().into_val(&env),
                    position_id.into_val(&env),
                    pos.claimable_payout.into_val(&env),
                ],
            ),
        );
        let payout = pos.claimable_payout;
        pos.status = PositionStatus::Claimed;
        set_position(&env, &pos);
        env.events().publish(
            (Symbol::new(&env, "payout_claimed"), position_id),
            (user, payout),
        );
    }

    pub fn withdraw_principal(env: Env, user: Address, position_id: u64) {
        let cfg = require_live(&env);
        user.require_auth();
        let mut pos = position(&env, position_id);
        if pos.owner != user {
            panic_with_error!(&env, EngineError::NotOwner);
        }
        if pos.principal_withdrawn {
            panic_with_error!(&env, EngineError::AlreadyWithdrawn);
        }
        if pos.status != PositionStatus::Expired
            && pos.status != PositionStatus::Claimed
            && pos.status != PositionStatus::Triggered
        {
            panic_with_error!(&env, EngineError::InvalidStatus);
        }

        invoke_unit(
            &env,
            &cfg.protected_balance_vault,
            "withdraw_to",
            Vec::from_array(
                &env,
                [
                    env.current_contract_address().into_val(&env),
                    user.clone().into_val(&env),
                    pos.protected_asset.clone().into_val(&env),
                    pos.protected_amount.into_val(&env),
                    position_id.into_val(&env),
                ],
            ),
        );
        pos.principal_withdrawn = true;
        set_position(&env, &pos);
        env.events().publish(
            (Symbol::new(&env, "principal_withdrawn"), position_id),
            (user, pos.protected_amount),
        );
    }

    pub fn expire_position(env: Env, position_id: u64) {
        let cfg = require_live(&env);
        let mut pos = position(&env, position_id);
        if pos.status != PositionStatus::Active {
            panic_with_error!(&env, EngineError::InvalidStatus);
        }
        if env.ledger().timestamp() <= pos.expiry_time {
            panic_with_error!(&env, EngineError::NotExpired);
        }

        invoke_unit(
            &env,
            &cfg.reserve_vault,
            "release_payout_capacity",
            Vec::from_array(
                &env,
                [
                    env.current_contract_address().into_val(&env),
                    pos.payout_asset.clone().into_val(&env),
                    position_id.into_val(&env),
                ],
            ),
        );
        pos.status = PositionStatus::Expired;
        set_position(&env, &pos);
        env.events()
            .publish((Symbol::new(&env, "position_expired"), position_id), ());
    }

    pub fn get_position(env: Env, position_id: u64) -> Position {
        position(&env, position_id)
    }

    pub fn get_user_positions(env: Env, user: Address) -> Vec<u64> {
        user_positions(&env, &user)
    }

    pub fn get_fee_quote(env: Env, protected_amount: i128) -> i128 {
        if protected_amount <= 0 {
            panic_with_error!(&env, EngineError::InvalidAmount);
        }
        amount_by_bps(protected_amount, config(&env).fee_bps)
    }

    pub fn get_fee_quote_for_duration(
        env: Env,
        protected_amount: i128,
        duration_seconds: u64,
    ) -> i128 {
        if protected_amount <= 0 {
            panic_with_error!(&env, EngineError::InvalidAmount);
        }
        amount_by_bps(
            protected_amount,
            fee_bps_for_duration(&env, duration_seconds),
        )
    }

    pub fn get_max_payout(env: Env, protected_amount: i128) -> i128 {
        if protected_amount <= 0 {
            panic_with_error!(&env, EngineError::InvalidAmount);
        }
        amount_by_bps(protected_amount, config(&env).max_payout_bps)
    }

    pub fn get_config(env: Env) -> Config {
        config(&env)
    }

    pub fn set_fee_bps(env: Env, admin: Address, fee_bps: u32) {
        require_admin(&env, &admin);
        validate_fee_bps(&env, fee_bps);
        let mut cfg = config(&env);
        cfg.fee_bps = fee_bps;
        set_config(&env, &cfg);
    }

    pub fn set_max_payout_bps(env: Env, admin: Address, max_payout_bps: u32) {
        require_admin(&env, &admin);
        validate_max_payout_bps(&env, max_payout_bps);
        let mut cfg = config(&env);
        cfg.max_payout_bps = max_payout_bps;
        set_config(&env, &cfg);
    }

    pub fn set_max_oracle_age_seconds(env: Env, admin: Address, max_oracle_age_seconds: u64) {
        require_admin(&env, &admin);
        if max_oracle_age_seconds == 0 {
            panic_with_error!(&env, EngineError::StaleOraclePrice);
        }
        let mut cfg = config(&env);
        cfg.max_oracle_age_seconds = max_oracle_age_seconds;
        set_config(&env, &cfg);
    }

    pub fn pause(env: Env, admin: Address) {
        require_admin(&env, &admin);
        let mut cfg = config(&env);
        cfg.paused = true;
        set_config(&env, &cfg);
    }

    pub fn unpause(env: Env, admin: Address) {
        require_admin(&env, &admin);
        let mut cfg = config(&env);
        cfg.paused = false;
        set_config(&env, &cfg);
    }
}

#[cfg(test)]
mod test;
