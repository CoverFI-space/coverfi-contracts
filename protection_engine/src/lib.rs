#![no_std]

use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, panic_with_error, token,
    Address, Env, Symbol, Vec,
};

const BPS: i128 = 10_000;
const PRICE_SCALE: i128 = 100_000_000;
const MIN_NOTIONAL: i128 = 10_000_000;
const MIN_RISK_PREMIUM: i128 = 1_000_000;
const AUTOMATION_FEE: i128 = 500_000;
const SETTLEMENT_KEEPER_REWARD: i128 = 100_000;
const SAFETY_MARGIN_BPS: u32 = 20;
const MAX_PREMIUM_BPS: u32 = 1_000;
const MAX_UTILIZATION_BPS: u32 = 9_000;
const MAX_CONCENTRATION_BPS: u32 = 7_500;
const EXPIRY_OBSERVATION_MAX_AGE: u64 = 600;
const ONE_DAY: u64 = 86_400;
const MAX_DURATION: u64 = 30 * ONE_DAY;
const INSTANCE_TTL_THRESHOLD: u32 = 30 * 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 120 * 17_280;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceObservation {
    pub price: i128,
    pub timestamp: u64,
}

#[contractclient(name = "OracleAdapterClient")]
pub trait OracleAdapterInterface {
    fn get_market(env: Env) -> (Address, Address);
    fn get_fresh_price(env: Env, asset: Address) -> Option<PriceObservation>;
    fn get_price_at_or_before(
        env: Env,
        asset: Address,
        timestamp: u64,
        max_age_seconds: u64,
    ) -> Option<PriceObservation>;
    fn get_volatility_bps(env: Env, asset: Address) -> u32;
}

#[contractclient(name = "ProtectedBalanceVaultClient")]
pub trait ProtectedBalanceVaultInterface {
    fn deposit_from(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        amount: i128,
        position_id: u64,
    );
    fn withdraw_to(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        amount: i128,
        position_id: u64,
    );
}

#[contractclient(name = "ReserveVaultClient")]
pub trait ReserveVaultInterface {
    fn collect_premium_from(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        risk_premium: i128,
        automation_fee: i128,
        position_id: u64,
        partner: Option<Address>,
    );
    fn lock_payout_capacity(
        env: Env,
        engine: Address,
        protected_asset: Address,
        payout_token: Address,
        position_id: u64,
        amount: i128,
    );
    fn settle_position(env: Env, engine: Address, owner: Address, position_id: u64, payout: i128);
    fn claim_payout(env: Env, engine: Address, owner: Address, position_id: u64) -> i128;
    fn pay_keeper(
        env: Env,
        caller: Address,
        token: Address,
        keeper: Address,
        action: Symbol,
        action_id: u64,
        amount: i128,
    ) -> bool;
    fn get_projected_utilization_bps(env: Env, token: Address, additional: i128) -> u32;
    fn get_projected_concentration_bps(
        env: Env,
        protected_asset: Address,
        payout_token: Address,
        additional: i128,
    ) -> u32;
    fn get_protocol_fee_bps(env: Env) -> i128;
}

#[contract]
pub struct ProtectionEngine;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PositionStatus {
    Active,
    AwaitingOracle,
    SettledNoPayout,
    Claimable,
    Claimed,
    PrincipalWithdrawn,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectionQuote {
    pub entry_price: i128,
    pub notional: i128,
    pub maximum_payout: i128,
    pub base_premium: i128,
    pub volatility_surcharge: i128,
    pub utilization_surcharge: i128,
    pub concentration_surcharge: i128,
    pub safety_margin: i128,
    pub risk_premium: i128,
    pub protocol_commission: i128,
    pub automation_fee: i128,
    pub total_due: i128,
    pub utilization_bps: u32,
    pub concentration_bps: u32,
    pub volatility_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settlement {
    pub position_id: u64,
    pub settlement_price: i128,
    pub payout: i128,
    pub status: PositionStatus,
    pub keeper_rewarded: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub id: u64,
    pub owner: Address,
    pub partner: Option<Address>,
    pub protected_asset: Address,
    pub payout_asset: Address,
    pub protected_amount: i128,
    pub notional: i128,
    pub maximum_payout: i128,
    pub risk_premium: i128,
    pub automation_fee: i128,
    pub fee_paid: i128,
    pub entry_price: i128,
    pub settlement_price: i128,
    pub start_time: u64,
    pub expiry_time: u64,
    pub status: PositionStatus,
    pub claimable_payout: i128,
    pub payout_claimed: bool,
    pub principal_withdrawn: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub admin: Address,
    pub protected_balance_vault: Address,
    pub reserve_vault: Address,
    pub oracle_adapter: Address,
    pub protected_asset: Address,
    pub payout_asset: Address,
    pub max_payout_bps: u32,
    pub max_oracle_age_seconds: u64,
    pub paused: bool,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Config,
    PendingAdmin,
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
    InvalidMaxPayoutBps = 5,
    InvalidAmount = 6,
    InvalidDuration = 7,
    InvalidPrice = 8,
    PositionNotFound = 9,
    NotOwner = 10,
    InvalidStatus = 11,
    NotExpired = 12,
    AlreadyWithdrawn = 13,
    NothingToClaim = 14,
    StaleOraclePrice = 15,
    UnsupportedMarket = 16,
    UnsafeUtilization = 17,
    ConcentrationLimit = 18,
    MathOverflow = 19,
    NotionalTooSmall = 20,
    InvalidTokenDecimals = 22,
    OracleMarketMismatch = 23,
}

fn config(env: &Env) -> Config {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .unwrap_or_else(|| panic_with_error!(env, EngineError::NotInitialized))
}

fn set_config(env: &Env, cfg: &Config) {
    env.storage().instance().set(&DataKey::Config, cfg);
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != config(env).admin {
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

fn validate_market(env: &Env, cfg: &Config, protected_asset: &Address, payout_asset: &Address) {
    if *protected_asset != cfg.protected_asset || *payout_asset != cfg.payout_asset {
        panic_with_error!(env, EngineError::UnsupportedMarket);
    }
}

fn position(env: &Env, position_id: u64) -> Position {
    env.storage()
        .persistent()
        .get(&DataKey::Position(position_id))
        .unwrap_or_else(|| panic_with_error!(env, EngineError::PositionNotFound))
}

fn set_position(env: &Env, value: &Position) {
    env.storage()
        .persistent()
        .set(&DataKey::Position(value.id), value);
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
    let mut values = user_positions(env, user);
    values.push_back(position_id);
    env.storage()
        .persistent()
        .set(&DataKey::UserPositions(user.clone()), &values);
}

fn safe_mul_div(env: &Env, left: i128, right: i128, divisor: i128) -> i128 {
    if divisor <= 0 {
        panic_with_error!(env, EngineError::MathOverflow);
    }
    left.checked_mul(right)
        .and_then(|value| value.checked_div(divisor))
        .unwrap_or_else(|| panic_with_error!(env, EngineError::MathOverflow))
}

fn amount_by_bps(env: &Env, amount: i128, bps: u32) -> i128 {
    safe_mul_div(env, amount, i128::from(bps), BPS)
}

fn amount_by_i128_bps(env: &Env, amount: i128, bps: i128) -> i128 {
    safe_mul_div(env, amount, bps, BPS)
}

fn base_bps(env: &Env, duration: u64) -> u32 {
    if duration == 0 || duration > MAX_DURATION {
        panic_with_error!(env, EngineError::InvalidDuration);
    }
    if duration <= ONE_DAY {
        30
    } else if duration <= 7 * ONE_DAY {
        100
    } else if duration <= 14 * ONE_DAY {
        150
    } else {
        250
    }
}

fn volatility_surcharge_bps(value: u32) -> u32 {
    if value < 200 {
        0
    } else if value < 500 {
        25
    } else if value < 1_000 {
        75
    } else {
        150
    }
}

fn utilization_surcharge_bps(env: &Env, value: u32) -> u32 {
    if value >= MAX_UTILIZATION_BPS {
        panic_with_error!(env, EngineError::UnsafeUtilization);
    }
    if value < 5_000 {
        0
    } else if value < 7_500 {
        25
    } else {
        100
    }
}

fn concentration_surcharge_bps(env: &Env, value: u32) -> u32 {
    if value > MAX_CONCENTRATION_BPS {
        panic_with_error!(env, EngineError::ConcentrationLimit);
    }
    if value < 4_000 {
        0
    } else if value < 6_000 {
        25
    } else {
        75
    }
}

fn fresh_entry_observation(env: &Env, cfg: &Config) -> PriceObservation {
    let observation = OracleAdapterClient::new(env, &cfg.oracle_adapter)
        .get_fresh_price(&cfg.protected_asset)
        .unwrap_or_else(|| panic_with_error!(env, EngineError::StaleOraclePrice));
    if observation.price <= 0
        || observation.timestamp > env.ledger().timestamp()
        || env.ledger().timestamp() - observation.timestamp > cfg.max_oracle_age_seconds
    {
        panic_with_error!(env, EngineError::StaleOraclePrice);
    }
    observation
}

fn quote(
    env: &Env,
    cfg: &Config,
    protected_asset: &Address,
    payout_asset: &Address,
    protected_amount: i128,
    duration_seconds: u64,
) -> ProtectionQuote {
    validate_market(env, cfg, protected_asset, payout_asset);
    if protected_amount <= 0 {
        panic_with_error!(env, EngineError::InvalidAmount);
    }
    let base_rate = base_bps(env, duration_seconds);
    let observation = fresh_entry_observation(env, cfg);
    let notional = safe_mul_div(env, protected_amount, observation.price, PRICE_SCALE);
    if notional < MIN_NOTIONAL {
        panic_with_error!(env, EngineError::NotionalTooSmall);
    }
    let maximum_payout = amount_by_bps(env, notional, cfg.max_payout_bps);
    let reserve = ReserveVaultClient::new(env, &cfg.reserve_vault);
    let utilization_bps = reserve.get_projected_utilization_bps(payout_asset, &maximum_payout);
    let concentration_bps =
        reserve.get_projected_concentration_bps(protected_asset, payout_asset, &maximum_payout);
    let protocol_fee_bps = reserve.get_protocol_fee_bps();
    let volatility_bps =
        OracleAdapterClient::new(env, &cfg.oracle_adapter).get_volatility_bps(protected_asset);
    let base_premium = amount_by_bps(env, notional, base_rate);
    let volatility_surcharge =
        amount_by_bps(env, notional, volatility_surcharge_bps(volatility_bps));
    let utilization_surcharge = amount_by_bps(
        env,
        notional,
        utilization_surcharge_bps(env, utilization_bps),
    );
    let concentration_surcharge = amount_by_bps(
        env,
        notional,
        concentration_surcharge_bps(env, concentration_bps),
    );
    let safety_margin = amount_by_bps(env, notional, SAFETY_MARGIN_BPS);
    let raw = base_premium
        .checked_add(volatility_surcharge)
        .and_then(|value| value.checked_add(utilization_surcharge))
        .and_then(|value| value.checked_add(concentration_surcharge))
        .and_then(|value| value.checked_add(safety_margin))
        .unwrap_or_else(|| panic_with_error!(env, EngineError::MathOverflow));
    let cap = amount_by_bps(env, notional, MAX_PREMIUM_BPS);
    let risk_premium = raw.max(MIN_RISK_PREMIUM).min(cap);
    let total_due = risk_premium
        .checked_add(AUTOMATION_FEE)
        .unwrap_or_else(|| panic_with_error!(env, EngineError::MathOverflow));

    ProtectionQuote {
        entry_price: observation.price,
        notional,
        maximum_payout,
        base_premium,
        volatility_surcharge,
        utilization_surcharge,
        concentration_surcharge,
        safety_margin,
        risk_premium,
        protocol_commission: amount_by_i128_bps(env, risk_premium, protocol_fee_bps),
        automation_fee: AUTOMATION_FEE,
        total_due,
        utilization_bps,
        concentration_bps,
        volatility_bps,
    }
}

#[contractimpl]
impl ProtectionEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        env: Env,
        admin: Address,
        protected_balance_vault: Address,
        reserve_vault: Address,
        oracle_adapter: Address,
        protected_asset: Address,
        payout_asset: Address,
        max_payout_bps: u32,
        max_oracle_age_seconds: u64,
    ) {
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, EngineError::AlreadyInitialized);
        }
        admin.require_auth();
        if max_payout_bps == 0
            || max_payout_bps > 1_000
            || max_oracle_age_seconds == 0
            || protected_asset == payout_asset
        {
            panic_with_error!(&env, EngineError::InvalidMaxPayoutBps);
        }
        // Amounts use Stellar's seven-decimal token units; prices are payout
        // tokens per protected token, scaled by 1e8. No same-token depeg payout.
        if token::Client::new(&env, &protected_asset).decimals() != 7
            || token::Client::new(&env, &payout_asset).decimals() != 7
        {
            panic_with_error!(&env, EngineError::InvalidTokenDecimals);
        }
        let market = OracleAdapterClient::new(&env, &oracle_adapter).get_market();
        if market != (protected_asset.clone(), payout_asset.clone()) {
            panic_with_error!(&env, EngineError::OracleMarketMismatch);
        }
        set_config(
            &env,
            &Config {
                admin,
                protected_balance_vault,
                reserve_vault,
                oracle_adapter,
                protected_asset,
                payout_asset,
                max_payout_bps,
                max_oracle_age_seconds,
                paused: false,
            },
        );
        env.storage()
            .instance()
            .set(&DataKey::NextPositionId, &1_u64);
    }

    pub fn quote_position(
        env: Env,
        protected_asset: Address,
        payout_asset: Address,
        protected_amount: i128,
        duration_seconds: u64,
    ) -> ProtectionQuote {
        let cfg = require_live(&env);
        quote(
            &env,
            &cfg,
            &protected_asset,
            &payout_asset,
            protected_amount,
            duration_seconds,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_position(
        env: Env,
        user: Address,
        protected_asset: Address,
        payout_asset: Address,
        protected_amount: i128,
        duration_seconds: u64,
        partner: Option<Address>,
    ) -> u64 {
        let cfg = require_live(&env);
        user.require_auth();
        let priced = quote(
            &env,
            &cfg,
            &protected_asset,
            &payout_asset,
            protected_amount,
            duration_seconds,
        );
        let position_id = next_position_id(&env);
        let engine = env.current_contract_address();

        ProtectedBalanceVaultClient::new(&env, &cfg.protected_balance_vault).deposit_from(
            &engine,
            &user,
            &protected_asset,
            &protected_amount,
            &position_id,
        );
        ReserveVaultClient::new(&env, &cfg.reserve_vault).collect_premium_from(
            &engine,
            &user,
            &payout_asset,
            &priced.risk_premium,
            &priced.automation_fee,
            &position_id,
            &partner,
        );
        ReserveVaultClient::new(&env, &cfg.reserve_vault).lock_payout_capacity(
            &engine,
            &protected_asset,
            &payout_asset,
            &position_id,
            &priced.maximum_payout,
        );

        let start_time = env.ledger().timestamp();
        let record = Position {
            id: position_id,
            owner: user.clone(),
            partner,
            protected_asset: protected_asset.clone(),
            payout_asset,
            protected_amount,
            notional: priced.notional,
            maximum_payout: priced.maximum_payout,
            risk_premium: priced.risk_premium,
            automation_fee: priced.automation_fee,
            fee_paid: priced.total_due,
            entry_price: priced.entry_price,
            settlement_price: 0,
            start_time,
            expiry_time: start_time + duration_seconds,
            status: PositionStatus::Active,
            claimable_payout: 0,
            payout_claimed: false,
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
                priced.maximum_payout,
                priced.risk_premium,
                record.expiry_time,
            ),
        );
        position_id
    }

    pub fn settle_position(env: Env, caller: Address, position_id: u64) -> Settlement {
        caller.require_auth();
        let cfg = config(&env);
        let mut value = position(&env, position_id);
        if value.status != PositionStatus::Active && value.status != PositionStatus::AwaitingOracle
        {
            panic_with_error!(&env, EngineError::InvalidStatus);
        }
        if env.ledger().timestamp() < value.expiry_time {
            panic_with_error!(&env, EngineError::NotExpired);
        }
        let maybe_observation = OracleAdapterClient::new(&env, &cfg.oracle_adapter)
            .get_price_at_or_before(
                &value.protected_asset,
                &value.expiry_time,
                &EXPIRY_OBSERVATION_MAX_AGE,
            );
        let observation = if let Some(observation) = maybe_observation {
            observation
        } else {
            value.status = PositionStatus::AwaitingOracle;
            set_position(&env, &value);
            env.events().publish(
                (Symbol::new(&env, "awaiting_oracle"), position_id),
                value.expiry_time,
            );
            return Settlement {
                position_id,
                settlement_price: 0,
                payout: 0,
                status: PositionStatus::AwaitingOracle,
                keeper_rewarded: false,
            };
        };
        if observation.price <= 0
            || observation.timestamp > value.expiry_time
            || value.expiry_time - observation.timestamp > EXPIRY_OBSERVATION_MAX_AGE
        {
            panic_with_error!(&env, EngineError::InvalidPrice);
        }
        let payout = if observation.price >= value.entry_price {
            0
        } else {
            let loss = value.entry_price - observation.price;
            safe_mul_div(&env, value.protected_amount, loss, PRICE_SCALE).min(value.maximum_payout)
        };
        ReserveVaultClient::new(&env, &cfg.reserve_vault).settle_position(
            &env.current_contract_address(),
            &value.owner,
            &position_id,
            &payout,
        );
        value.settlement_price = observation.price;
        value.claimable_payout = payout;
        value.status = if payout > 0 {
            PositionStatus::Claimable
        } else {
            PositionStatus::SettledNoPayout
        };
        set_position(&env, &value);
        let keeper_rewarded = ReserveVaultClient::new(&env, &cfg.reserve_vault).pay_keeper(
            &env.current_contract_address(),
            &value.payout_asset,
            &caller,
            &Symbol::new(&env, "settlement"),
            &position_id,
            &SETTLEMENT_KEEPER_REWARD,
        );
        env.events().publish(
            (Symbol::new(&env, "position_settled"), position_id),
            (observation.price, payout, caller.clone(), keeper_rewarded),
        );
        Settlement {
            position_id,
            settlement_price: observation.price,
            payout,
            status: value.status,
            keeper_rewarded,
        }
    }

    pub fn claim_payout(env: Env, user: Address, position_id: u64) -> i128 {
        user.require_auth();
        let cfg = config(&env);
        let mut value = position(&env, position_id);
        if value.owner != user {
            panic_with_error!(&env, EngineError::NotOwner);
        }
        if value.status != PositionStatus::Claimable || value.claimable_payout <= 0 {
            panic_with_error!(&env, EngineError::NothingToClaim);
        }
        let amount = ReserveVaultClient::new(&env, &cfg.reserve_vault).claim_payout(
            &env.current_contract_address(),
            &user,
            &position_id,
        );
        if amount != value.claimable_payout {
            panic_with_error!(&env, EngineError::MathOverflow);
        }
        value.payout_claimed = true;
        value.status = if value.principal_withdrawn {
            PositionStatus::PrincipalWithdrawn
        } else {
            PositionStatus::Claimed
        };
        set_position(&env, &value);
        env.events().publish(
            (Symbol::new(&env, "payout_claimed"), position_id),
            (user, amount),
        );
        amount
    }

    pub fn withdraw_principal(env: Env, user: Address, position_id: u64) -> i128 {
        user.require_auth();
        let cfg = config(&env);
        let mut value = position(&env, position_id);
        if value.owner != user {
            panic_with_error!(&env, EngineError::NotOwner);
        }
        if value.principal_withdrawn {
            panic_with_error!(&env, EngineError::AlreadyWithdrawn);
        }
        if value.status != PositionStatus::SettledNoPayout
            && value.status != PositionStatus::Claimable
            && value.status != PositionStatus::Claimed
        {
            panic_with_error!(&env, EngineError::InvalidStatus);
        }
        ProtectedBalanceVaultClient::new(&env, &cfg.protected_balance_vault).withdraw_to(
            &env.current_contract_address(),
            &user,
            &value.protected_asset,
            &value.protected_amount,
            &position_id,
        );
        value.principal_withdrawn = true;
        if value.status != PositionStatus::Claimable {
            value.status = PositionStatus::PrincipalWithdrawn;
        }
        set_position(&env, &value);
        env.events().publish(
            (Symbol::new(&env, "principal_withdrawn"), position_id),
            (user, value.protected_amount),
        );
        value.protected_amount
    }

    pub fn get_position(env: Env, position_id: u64) -> Position {
        position(&env, position_id)
    }

    pub fn get_user_positions(env: Env, user: Address) -> Vec<u64> {
        user_positions(&env, &user)
    }

    pub fn get_config(env: Env) -> Config {
        config(&env)
    }

    pub fn propose_admin(env: Env, admin: Address, pending: Address) {
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &pending);
    }

    pub fn accept_admin(env: Env, pending: Address) {
        pending.require_auth();
        let expected: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .unwrap_or_else(|| panic_with_error!(&env, EngineError::NotAdmin));
        if pending != expected {
            panic_with_error!(&env, EngineError::NotAdmin);
        }
        let mut cfg = config(&env);
        cfg.admin = pending;
        set_config(&env, &cfg);
        env.storage().instance().remove(&DataKey::PendingAdmin);
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

    pub fn extend_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
    }
}

#[cfg(test)]
mod test;
