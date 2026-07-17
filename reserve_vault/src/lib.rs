#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Symbol, Vec,
};

const BPS_DENOMINATOR: i128 = 10_000;
const DEFAULT_FLOOR_BPS: u32 = 2_000;
const DEFAULT_DRAIN_BPS: u32 = 5_000;

#[contract]
pub struct ReserveVault;

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Engine,
    FloorBps,
    DrainBps,
    CurrentEpoch,
    TotalReserve(Address),
    LockedReserve(Address),
    ReservedReserve(Address),
    PositionLock(u64),
    PositionToken(u64),
    Claim(u64),
    UserClaims(Address),
    EpochTotal(u64, Address),
    EpochAllocated(u64, Address),
    TotalPremiums(Address),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRecord {
    pub id: u64,
    pub owner: Address,
    pub token: Address,
    pub approved_amount: i128,
    pub withdrawn_amount: i128,
    pub epoch_id: u64,
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
    NotAdmin = 9,
    InvalidBps = 10,
    ClaimNotFound = 11,
    NotOwner = 12,
    NothingToWithdraw = 13,
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

fn admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, ReserveError::NotInitialized))
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != admin(env) {
        panic_with_error!(env, ReserveError::NotAdmin);
    }
}

fn read_i128(env: &Env, key: &DataKey) -> i128 {
    env.storage().persistent().get(key).unwrap_or(0)
}

fn read_u64(env: &Env, key: &DataKey) -> u64 {
    env.storage().instance().get(key).unwrap_or(0)
}

fn current_epoch(env: &Env) -> u64 {
    read_u64(env, &DataKey::CurrentEpoch)
}

fn floor_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::FloorBps)
        .unwrap_or(DEFAULT_FLOOR_BPS)
}

fn drain_bps(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::DrainBps)
        .unwrap_or(DEFAULT_DRAIN_BPS)
}

fn amount_by_bps(amount: i128, bps: u32) -> i128 {
    amount * i128::from(bps) / BPS_DENOMINATOR
}

fn allocation_budget(env: &Env, token: Address, requested_amount: i128) -> i128 {
    if requested_amount <= 0 {
        return 0;
    }

    let total_reserve = ReserveVault::get_total_reserve(env.clone(), token.clone());
    let locked_reserve = ReserveVault::get_locked_reserve(env.clone(), token.clone());
    let reserved_reserve = ReserveVault::get_reserved_reserve(env.clone(), token.clone());
    let free_liquidity = total_reserve - locked_reserve - reserved_reserve;
    let floor = amount_by_bps(total_reserve, floor_bps(env));
    let surplus = if free_liquidity > floor {
        free_liquidity - floor
    } else {
        0
    };
    let drain_budget = amount_by_bps(surplus, drain_bps(env));

    if requested_amount < drain_budget {
        requested_amount
    } else {
        drain_budget
    }
}

fn allocate_epoch_budget(env: &Env, epoch_id: u64, token: Address) -> i128 {
    let total_key = DataKey::EpochTotal(epoch_id, token.clone());
    let allocated_key = DataKey::EpochAllocated(epoch_id, token.clone());
    let epoch_total = read_i128(env, &total_key);
    let epoch_allocated = read_i128(env, &allocated_key);
    let unpaid_epoch_total = epoch_total - epoch_allocated;
    let budget = allocation_budget(env, token.clone(), unpaid_epoch_total);

    if budget <= 0 {
        return 0;
    }

    env.storage()
        .persistent()
        .set(&allocated_key, &(epoch_allocated + budget));
    let reserved_key = DataKey::ReservedReserve(token);
    env.storage()
        .persistent()
        .set(&reserved_key, &(read_i128(env, &reserved_key) + budget));
    budget
}

fn claim(env: &Env, claim_id: u64) -> ClaimRecord {
    env.storage()
        .persistent()
        .get(&DataKey::Claim(claim_id))
        .unwrap_or_else(|| panic_with_error!(env, ReserveError::ClaimNotFound))
}

fn set_claim(env: &Env, record: &ClaimRecord) {
    env.storage()
        .persistent()
        .set(&DataKey::Claim(record.id), record);
}

fn user_claims(env: &Env, user: &Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::UserClaims(user.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

fn save_user_claim(env: &Env, user: &Address, claim_id: u64) {
    let mut ids = user_claims(env, user);
    ids.push_back(claim_id);
    env.storage()
        .persistent()
        .set(&DataKey::UserClaims(user.clone()), &ids);
}

#[contractimpl]
impl ReserveVault {
    pub fn initialize(env: Env, admin: Address, engine: Address) {
        if env.storage().instance().has(&DataKey::Engine) {
            panic_with_error!(&env, ReserveError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Engine, &engine);
        env.storage()
            .instance()
            .set(&DataKey::FloorBps, &DEFAULT_FLOOR_BPS);
        env.storage()
            .instance()
            .set(&DataKey::DrainBps, &DEFAULT_DRAIN_BPS);
        env.storage().instance().set(&DataKey::CurrentEpoch, &1_u64);
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

    pub fn collect_premium_from(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        amount: i128,
        position_id: u64,
    ) {
        require_engine(&env, &engine);
        if amount <= 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }

        token::Client::new(&env, &token).transfer(&user, &env.current_contract_address(), &amount);

        let total_key = DataKey::TotalReserve(token.clone());
        let premium_key = DataKey::TotalPremiums(token.clone());
        let next_total = read_i128(&env, &total_key) + amount;
        let actual_balance =
            token::Client::new(&env, &token).balance(&env.current_contract_address());
        if actual_balance < next_total {
            panic_with_error!(&env, ReserveError::AccountingBalanceTooLow);
        }

        env.storage().persistent().set(&total_key, &next_total);
        env.storage()
            .persistent()
            .set(&premium_key, &(read_i128(&env, &premium_key) + amount));
        env.events().publish(
            (Symbol::new(&env, "premium_collected"), position_id),
            (user, token, amount),
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

        let locked_key = DataKey::LockedReserve(token);
        env.storage().persistent().set(
            &locked_key,
            &(read_i128(&env, &locked_key) - locked_for_position),
        );
        env.storage()
            .persistent()
            .set(&DataKey::PositionLock(position_id), &0_i128);

        let epoch_id = current_epoch(&env);
        let claim_record = ClaimRecord {
            id: position_id,
            owner: user.clone(),
            token: stored_token.clone(),
            approved_amount: payout_amount,
            withdrawn_amount: 0,
            epoch_id,
        };
        set_claim(&env, &claim_record);
        save_user_claim(&env, &user, position_id);

        let epoch_key = DataKey::EpochTotal(epoch_id, stored_token);
        env.storage()
            .persistent()
            .set(&epoch_key, &(read_i128(&env, &epoch_key) + payout_amount));
        env.events().publish(
            (Symbol::new(&env, "claim_approved"), position_id),
            (user, payout_amount),
        );
    }

    pub fn close_epoch(env: Env, admin: Address, token: Address) -> i128 {
        require_admin(&env, &admin);
        let epoch_id = current_epoch(&env);
        let budget = allocate_epoch_budget(&env, epoch_id, token.clone());
        env.storage()
            .instance()
            .set(&DataKey::CurrentEpoch, &(epoch_id + 1));
        env.events().publish(
            (Symbol::new(&env, "epoch_closed"), epoch_id),
            (token, budget, floor_bps(&env), drain_bps(&env)),
        );
        budget
    }

    pub fn allocate_epoch(env: Env, admin: Address, epoch_id: u64, token: Address) -> i128 {
        require_admin(&env, &admin);
        let budget = allocate_epoch_budget(&env, epoch_id, token.clone());
        env.events().publish(
            (Symbol::new(&env, "epoch_allocated"), epoch_id),
            (token, budget, floor_bps(&env), drain_bps(&env)),
        );
        budget
    }

    pub fn withdraw_reserved(env: Env, user: Address, claim_id: u64) -> i128 {
        user.require_auth();
        let mut record = claim(&env, claim_id);
        if record.owner != user {
            panic_with_error!(&env, ReserveError::NotOwner);
        }
        let claimable = Self::quote_withdraw(env.clone(), claim_id);
        if claimable <= 0 {
            panic_with_error!(&env, ReserveError::NothingToWithdraw);
        }

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &user,
            &claimable,
        );
        record.withdrawn_amount += claimable;
        set_claim(&env, &record);

        let total_key = DataKey::TotalReserve(record.token.clone());
        let reserved_key = DataKey::ReservedReserve(record.token.clone());
        env.storage()
            .persistent()
            .set(&total_key, &(read_i128(&env, &total_key) - claimable));
        env.storage()
            .persistent()
            .set(&reserved_key, &(read_i128(&env, &reserved_key) - claimable));
        env.events().publish(
            (Symbol::new(&env, "reserved_withdrawn"), claim_id),
            (user, claimable),
        );
        claimable
    }

    pub fn get_total_reserve(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::TotalReserve(token))
    }

    pub fn get_locked_reserve(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::LockedReserve(token))
    }

    pub fn get_reserved_reserve(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::ReservedReserve(token))
    }

    pub fn get_total_premiums(env: Env, token: Address) -> i128 {
        read_i128(&env, &DataKey::TotalPremiums(token))
    }

    pub fn get_available_reserve(env: Env, token: Address) -> i128 {
        Self::get_total_reserve(env.clone(), token.clone())
            - Self::get_locked_reserve(env.clone(), token.clone())
            - Self::get_reserved_reserve(env, token)
    }

    pub fn get_locked_for_position(env: Env, position_id: u64) -> i128 {
        read_i128(&env, &DataKey::PositionLock(position_id))
    }

    pub fn get_claim(env: Env, claim_id: u64) -> ClaimRecord {
        claim(&env, claim_id)
    }

    pub fn get_user_claims(env: Env, user: Address) -> Vec<u64> {
        user_claims(&env, &user)
    }

    pub fn get_current_epoch(env: Env) -> u64 {
        current_epoch(&env)
    }

    pub fn get_floor_bps(env: Env) -> u32 {
        floor_bps(&env)
    }

    pub fn get_drain_bps(env: Env) -> u32 {
        drain_bps(&env)
    }

    pub fn get_epoch_total(env: Env, epoch_id: u64, token: Address) -> i128 {
        read_i128(&env, &DataKey::EpochTotal(epoch_id, token))
    }

    pub fn get_epoch_allocated(env: Env, epoch_id: u64, token: Address) -> i128 {
        read_i128(&env, &DataKey::EpochAllocated(epoch_id, token))
    }

    pub fn quote_withdraw(env: Env, claim_id: u64) -> i128 {
        let record = claim(&env, claim_id);
        let epoch_total = read_i128(
            &env,
            &DataKey::EpochTotal(record.epoch_id, record.token.clone()),
        );
        if epoch_total <= 0 {
            return 0;
        }
        let epoch_allocated = read_i128(
            &env,
            &DataKey::EpochAllocated(record.epoch_id, record.token.clone()),
        );
        let mut allocated_to_claim = record.approved_amount * epoch_allocated / epoch_total;
        if allocated_to_claim > record.approved_amount {
            allocated_to_claim = record.approved_amount;
        }
        if allocated_to_claim <= record.withdrawn_amount {
            return 0;
        }
        allocated_to_claim - record.withdrawn_amount
    }

    pub fn set_restricted_payout_policy(env: Env, admin: Address, floor_bps: u32, drain_bps: u32) {
        require_admin(&env, &admin);
        if floor_bps > 9_000 || drain_bps == 0 || drain_bps > 10_000 {
            panic_with_error!(&env, ReserveError::InvalidBps);
        }
        env.storage().instance().set(&DataKey::FloorBps, &floor_bps);
        env.storage().instance().set(&DataKey::DrainBps, &drain_bps);
        env.events().publish(
            (Symbol::new(&env, "restricted_payout_policy_updated"), admin),
            (floor_bps, drain_bps),
        );
    }
}

#[cfg(test)]
mod test;
