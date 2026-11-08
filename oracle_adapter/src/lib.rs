#![no_std]

use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, panic_with_error, Address,
    Env, Symbol, Vec,
};

const TARGET_DECIMALS: u32 = 8;
const MAX_OBSERVATIONS: u32 = 64;
// Historical minute buckets preserve expiry observations after the quote ring
// rolls over. Strictly increasing seconds bound each bucket to 60 entries.
const MAX_HISTORY_LOOKBACK: u64 = 3_600;
const INSTANCE_TTL_THRESHOLD: u32 = 30 * 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 120 * 17_280;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReflectorAsset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflectorPriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contractclient(name = "ReflectorClient")]
pub trait Reflector {
    fn lastprice(env: Env, caller: Address, asset: ReflectorAsset) -> Option<ReflectorPriceData>;
    fn price(
        env: Env,
        caller: Address,
        asset: ReflectorAsset,
        timestamp: u64,
    ) -> Option<ReflectorPriceData>;
}

#[contract]
pub struct OracleAdapter;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceObservation {
    pub price: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub admin: Address,
    pub source: Option<Address>,
    pub protected_asset: Address,
    pub payout_asset: Address,
    pub source_asset: Symbol,
    pub source_decimals: u32,
    pub max_age_seconds: u64,
    pub max_deviation_bps: u32,
    pub fallback_enabled: bool,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Config,
    PendingAdmin,
    Observations(Address),
    History(Address, u64),
    PublisherEpoch,
    Publishers,
    Publisher(Address),
    Submission(u64, Address, u64, Address),
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
    StalePrice = 6,
    FuturePrice = 7,
    ExcessiveDeviation = 8,
    NotPublisher = 9,
    DuplicateSubmission = 10,
    InvalidConfig = 11,
    UnsupportedAsset = 12,
    MathOverflow = 13,
}

fn config(env: &Env) -> OracleConfig {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .unwrap_or_else(|| panic_with_error!(env, OracleError::NotInitialized))
}

fn set_config(env: &Env, cfg: &OracleConfig) {
    env.storage().instance().set(&DataKey::Config, cfg);
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != config(env).admin {
        panic_with_error!(env, OracleError::NotAdmin);
    }
}

fn pow10(env: &Env, power: u32) -> i128 {
    if power > 18 {
        panic_with_error!(env, OracleError::InvalidConfig);
    }
    let mut value = 1_i128;
    let mut i = 0;
    while i < power {
        value *= 10;
        i += 1;
    }
    value
}

fn normalize(env: &Env, price: i128, decimals: u32) -> i128 {
    if price <= 0 {
        panic_with_error!(env, OracleError::InvalidPrice);
    }
    if decimals == TARGET_DECIMALS {
        price
    } else if decimals > TARGET_DECIMALS {
        price / pow10(env, decimals - TARGET_DECIMALS)
    } else {
        price
            .checked_mul(pow10(env, TARGET_DECIMALS - decimals))
            .unwrap_or_else(|| panic_with_error!(env, OracleError::MathOverflow))
    }
}

fn observations(env: &Env, asset: &Address) -> Vec<PriceObservation> {
    env.storage()
        .persistent()
        .get(&DataKey::Observations(asset.clone()))
        .unwrap_or(Vec::new(env))
}

fn require_asset(env: &Env, asset: &Address) {
    if *asset != config(env).protected_asset {
        panic_with_error!(env, OracleError::UnsupportedAsset);
    }
}

fn validate_observation(env: &Env, observation: &PriceObservation) {
    if observation.price <= 0 {
        panic_with_error!(env, OracleError::InvalidPrice);
    }
    let now = env.ledger().timestamp();
    if observation.timestamp > now {
        panic_with_error!(env, OracleError::FuturePrice);
    }
    if now - observation.timestamp > config(env).max_age_seconds {
        panic_with_error!(env, OracleError::StalePrice);
    }
}

fn save_observation(env: &Env, asset: &Address, observation: PriceObservation) -> bool {
    require_asset(env, asset);
    validate_observation(env, &observation);
    let cfg = config(env);
    let mut list = observations(env, asset);
    if let Some(last) = list.last() {
        if observation.timestamp <= last.timestamp {
            return false;
        }
        let delta = if observation.price > last.price {
            observation.price - last.price
        } else {
            last.price - observation.price
        };
        let deviation = delta
            .checked_mul(10_000)
            .and_then(|value| value.checked_div(last.price))
            .unwrap_or_else(|| panic_with_error!(env, OracleError::MathOverflow));
        if deviation > i128::from(cfg.max_deviation_bps) {
            panic_with_error!(env, OracleError::ExcessiveDeviation);
        }
    }
    if list.len() >= MAX_OBSERVATIONS {
        list.remove(0);
    }
    let history_key = DataKey::History(asset.clone(), observation.timestamp / 60);
    let mut history: Vec<PriceObservation> = env
        .storage()
        .persistent()
        .get(&history_key)
        .unwrap_or(Vec::new(env));
    history.push_back(observation.clone());
    env.storage().persistent().set(&history_key, &history);
    env.storage().persistent().extend_ttl(
        &history_key,
        INSTANCE_TTL_THRESHOLD,
        INSTANCE_TTL_EXTEND_TO,
    );
    list.push_back(observation.clone());
    env.storage()
        .persistent()
        .set(&DataKey::Observations(asset.clone()), &list);
    env.events().publish(
        (Symbol::new(env, "price_observed"), asset.clone()),
        (observation.price, observation.timestamp),
    );
    true
}

fn source_price(env: &Env, at: Option<u64>) -> Option<PriceObservation> {
    let cfg = config(env);
    let source_address = cfg.source?;
    let source_asset = ReflectorAsset::Other(cfg.source_asset);
    let caller = env.current_contract_address();
    let source = ReflectorClient::new(env, &source_address);
    let raw = if let Some(timestamp) = at {
        source.price(&caller, &source_asset, &timestamp)
    } else {
        source.lastprice(&caller, &source_asset)
    };
    raw.map(|value| PriceObservation {
        price: normalize(env, value.price, cfg.source_decimals),
        timestamp: value.timestamp,
    })
}

#[contractimpl]
impl OracleAdapter {
    pub fn initialize(
        env: Env,
        admin: Address,
        source: Option<Address>,
        source_asset: Symbol,
        source_decimals: u32,
        max_age_seconds: u64,
        max_deviation_bps: u32,
        protected_asset: Address,
        payout_asset: Address,
    ) {
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, OracleError::AlreadyInitialized);
        }
        admin.require_auth();
        if max_age_seconds == 0
            || max_age_seconds > MAX_HISTORY_LOOKBACK
            || max_deviation_bps == 0
            || max_deviation_bps > 10_000
            || source_decimals > 18
            || protected_asset == payout_asset
        {
            panic_with_error!(&env, OracleError::InvalidConfig);
        }
        set_config(
            &env,
            &OracleConfig {
                admin,
                source,
                protected_asset,
                payout_asset,
                source_asset,
                source_decimals,
                max_age_seconds,
                max_deviation_bps,
                fallback_enabled: false,
            },
        );
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
            .unwrap_or_else(|| panic_with_error!(&env, OracleError::NotAdmin));
        if expected != pending {
            panic_with_error!(&env, OracleError::NotAdmin);
        }
        let mut cfg = config(&env);
        cfg.admin = pending;
        set_config(&env, &cfg);
        env.storage().instance().remove(&DataKey::PendingAdmin);
    }

    pub fn configure_publishers(
        env: Env,
        admin: Address,
        publisher1: Address,
        publisher2: Address,
        publisher3: Address,
        enabled: bool,
    ) {
        require_admin(&env, &admin);
        if publisher1 == publisher2 || publisher1 == publisher3 || publisher2 == publisher3 {
            panic_with_error!(&env, OracleError::InvalidConfig);
        }

        let epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PublisherEpoch)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::PublisherEpoch, &(epoch + 1));
        let previous: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Publishers)
            .unwrap_or(Vec::new(&env));
        for publisher in previous.iter() {
            env.storage()
                .persistent()
                .set(&DataKey::Publisher(publisher), &false);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher1.clone()), &true);
        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher2.clone()), &true);
        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher3.clone()), &true);
        env.storage().persistent().set(
            &DataKey::Publishers,
            &soroban_sdk::vec![&env, publisher1, publisher2, publisher3],
        );
        let mut cfg = config(&env);
        cfg.fallback_enabled = enabled;
        set_config(&env, &cfg);
    }

    pub fn refresh(env: Env, _caller: Address, asset: Address) -> bool {
        let observation = source_price(&env, None)
            .unwrap_or_else(|| panic_with_error!(&env, OracleError::MissingPrice));
        save_observation(&env, &asset, observation)
    }

    pub fn refresh_at(env: Env, _caller: Address, asset: Address, timestamp: u64) -> bool {
        let observation = source_price(&env, Some(timestamp))
            .unwrap_or_else(|| panic_with_error!(&env, OracleError::MissingPrice));
        save_observation(&env, &asset, observation)
    }

    pub fn submit_fallback(
        env: Env,
        publisher: Address,
        asset: Address,
        price: i128,
        timestamp: u64,
    ) -> bool {
        publisher.require_auth();
        require_asset(&env, &asset);
        validate_observation(&env, &PriceObservation { price, timestamp });
        let cfg = config(&env);
        let approved: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Publisher(publisher.clone()))
            .unwrap_or(false);
        if !cfg.fallback_enabled || !approved {
            panic_with_error!(&env, OracleError::NotPublisher);
        }
        let epoch: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PublisherEpoch)
            .unwrap_or(0);
        let key = DataKey::Submission(epoch, asset.clone(), timestamp, publisher);
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, OracleError::DuplicateSubmission);
        }
        env.storage().persistent().set(&key, &price);

        let mut matches = 0_u32;
        let publishers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Publishers)
            .unwrap_or(Vec::new(&env));
        for configured_publisher in publishers.iter() {
            let submitted: Option<i128> = env.storage().persistent().get(&DataKey::Submission(
                epoch,
                asset.clone(),
                timestamp,
                configured_publisher,
            ));
            if submitted == Some(price) {
                matches += 1;
            }
        }
        if matches >= 2 {
            return save_observation(&env, &asset, PriceObservation { price, timestamp });
        }
        false
    }

    pub fn get_market(env: Env) -> (Address, Address) {
        let cfg = config(&env);
        (cfg.protected_asset, cfg.payout_asset)
    }

    pub fn get_latest(env: Env, asset: Address) -> Option<PriceObservation> {
        observations(&env, &asset).last()
    }

    pub fn get_fresh_price(env: Env, asset: Address) -> Option<PriceObservation> {
        let latest = observations(&env, &asset).last();
        if let Some(value) = latest {
            let now = env.ledger().timestamp();
            if value.timestamp <= now && now - value.timestamp <= config(&env).max_age_seconds {
                return Some(value);
            }
        }
        None
    }

    pub fn get_price_at_or_before(
        env: Env,
        asset: Address,
        timestamp: u64,
        max_age_seconds: u64,
    ) -> Option<PriceObservation> {
        require_asset(&env, &asset);
        if timestamp > env.ledger().timestamp() || max_age_seconds > MAX_HISTORY_LOOKBACK {
            return None;
        }
        let oldest = timestamp.saturating_sub(max_age_seconds);
        let mut bucket = timestamp / 60;
        loop {
            let list: Vec<PriceObservation> = env
                .storage()
                .persistent()
                .get(&DataKey::History(asset.clone(), bucket))
                .unwrap_or(Vec::new(&env));
            let mut index = list.len();
            while index > 0 {
                index -= 1;
                let value = list.get(index).unwrap();
                if value.timestamp <= timestamp && value.timestamp >= oldest {
                    return Some(value);
                }
            }
            if bucket == oldest / 60 {
                break;
            }
            bucket -= 1;
        }
        None
    }

    pub fn get_volatility_bps(env: Env, asset: Address) -> u32 {
        let list = observations(&env, &asset);
        if list.len() < 2 {
            return 0;
        }
        let first = list.first().unwrap();
        let last = list.last().unwrap();
        let delta = if last.price > first.price {
            last.price - first.price
        } else {
            first.price - last.price
        };
        delta
            .checked_mul(10_000)
            .and_then(|value| value.checked_div(first.price))
            .unwrap_or_else(|| panic_with_error!(&env, OracleError::MathOverflow))
            .min(10_000) as u32
    }

    pub fn get_config(env: Env) -> OracleConfig {
        config(&env)
    }

    pub fn extend_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
    }
}

#[cfg(test)]
mod test;
