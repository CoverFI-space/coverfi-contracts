#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    String, Symbol, Vec,
};

const LEASE_SECONDS: u64 = 30 * 86_400;
const GRACE_SECONDS: u64 = 7 * 86_400;
const PREMIUM_WINDOW: u64 = 21 * 86_400;
const MAX_LEASE_COUNT: u32 = 12;
const STANDARD_PRICE: i128 = 30_000_000;
const FOUR_CHAR_PRICE: i128 = 250_000_000;
const THREE_CHAR_PRICE: i128 = 1_000_000_000;
const INSTANCE_TTL_THRESHOLD: u32 = 30 * 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 120 * 17_280;

#[contract]
pub struct UsernameRegistry;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameRecord {
    pub name: String,
    pub owner: Address,
    pub registered_at: u64,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameQuote {
    pub base_fee: i128,
    pub scarcity_premium: i128,
    pub total_fee: i128,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryConfig {
    pub admin: Address,
    pub fee_token: Address,
    pub treasury: Address,
    pub automation: Address,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Config,
    PendingAdmin,
    Name(String),
    UserName(Address),
    SeenName(String),
    Names,
    TreasuryRevenue,
    AutomationRevenue,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RegistryError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    UsernameTaken = 4,
    UsernameNotFound = 5,
    AddressNotFound = 6,
    InvalidUsername = 7,
    ReservedUsername = 8,
    InvalidLeaseCount = 9,
    AlreadyHasUsername = 10,
    NotOwner = 11,
    RenewalWindowClosed = 12,
    InvalidPagination = 13,
}

fn config(env: &Env) -> RegistryConfig {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .unwrap_or_else(|| panic_with_error!(env, RegistryError::NotInitialized))
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != config(env).admin {
        panic_with_error!(env, RegistryError::NotAdmin);
    }
}

fn record(env: &Env, name: &String) -> Option<NameRecord> {
    env.storage().persistent().get(&DataKey::Name(name.clone()))
}

fn set_record(env: &Env, value: &NameRecord) {
    env.storage()
        .persistent()
        .set(&DataKey::Name(value.name.clone()), value);
}

fn validate_lease_count(env: &Env, lease_count: u32) {
    if lease_count == 0 || lease_count > MAX_LEASE_COUNT {
        panic_with_error!(env, RegistryError::InvalidLeaseCount);
    }
}

fn is_valid_username(username: &String) -> bool {
    let len = username.len() as usize;
    if !(3..=24).contains(&len) {
        return false;
    }
    let mut bytes = [0_u8; 24];
    username.copy_into_slice(&mut bytes[..len]);
    for byte in bytes.iter().take(len) {
        let lower = *byte >= b'a' && *byte <= b'z';
        let digit = *byte >= b'0' && *byte <= b'9';
        if !(lower || digit || *byte == b'_') {
            return false;
        }
    }
    true
}

fn is_reserved(env: &Env, username: &String) -> bool {
    username == &String::from_str(env, "coverfi")
        || username == &String::from_str(env, "admin")
        || username == &String::from_str(env, "support")
        || username == &String::from_str(env, "treasury")
        || username == &String::from_str(env, "official")
        || username == &String::from_str(env, "oracle")
        || username == &String::from_str(env, "stellar")
}

fn validate_name(env: &Env, username: &String) {
    if !is_valid_username(username) {
        panic_with_error!(env, RegistryError::InvalidUsername);
    }
    if is_reserved(env, username) {
        panic_with_error!(env, RegistryError::ReservedUsername);
    }
}

fn unit_price(username: &String) -> i128 {
    match username.len() {
        3 => THREE_CHAR_PRICE,
        4 => FOUR_CHAR_PRICE,
        _ => STANDARD_PRICE,
    }
}

fn public_after(value: &NameRecord) -> u64 {
    value.expires_at + GRACE_SECONDS
}

fn is_live_or_grace(env: &Env, value: &NameRecord) -> bool {
    env.ledger().timestamp() <= public_after(value)
}

fn scarcity_premium(env: &Env, username: &String) -> i128 {
    if let Some(existing) = record(env, username) {
        let available_at = public_after(&existing);
        let now = env.ledger().timestamp();
        if now > available_at && now < available_at + PREMIUM_WINDOW {
            return unit_price(username) * i128::from(available_at + PREMIUM_WINDOW - now)
                / i128::from(PREMIUM_WINDOW);
        }
    }
    0
}

fn quote(env: &Env, username: &String, lease_count: u32) -> NameQuote {
    validate_name(env, username);
    validate_lease_count(env, lease_count);
    let base_fee = unit_price(username) * i128::from(lease_count);
    let premium = scarcity_premium(env, username);
    NameQuote {
        base_fee,
        scarcity_premium: premium,
        total_fee: base_fee + premium,
        expires_at: env.ledger().timestamp() + LEASE_SECONDS * u64::from(lease_count),
    }
}

fn route_fee(env: &Env, payer: &Address, amount: i128) {
    let cfg = config(env);
    let treasury_amount = amount * 8_000 / 10_000;
    let automation_amount = amount - treasury_amount;
    let client = token::Client::new(env, &cfg.fee_token);
    client.transfer(payer, &cfg.treasury, &treasury_amount);
    client.transfer(payer, &cfg.automation, &automation_amount);
    let treasury_total: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::TreasuryRevenue)
        .unwrap_or(0);
    let automation_total: i128 = env
        .storage()
        .persistent()
        .get(&DataKey::AutomationRevenue)
        .unwrap_or(0);
    env.storage().persistent().set(
        &DataKey::TreasuryRevenue,
        &(treasury_total + treasury_amount),
    );
    env.storage().persistent().set(
        &DataKey::AutomationRevenue,
        &(automation_total + automation_amount),
    );
}

fn remember_name(env: &Env, username: &String) {
    let seen_key = DataKey::SeenName(username.clone());
    if env.storage().persistent().has(&seen_key) {
        return;
    }
    let mut names: Vec<String> = env
        .storage()
        .persistent()
        .get(&DataKey::Names)
        .unwrap_or(Vec::new(env));
    names.push_back(username.clone());
    env.storage().persistent().set(&DataKey::Names, &names);
    env.storage().persistent().set(&seen_key, &true);
}

fn clear_user_mapping_if_stale(env: &Env, user: &Address) {
    let key = DataKey::UserName(user.clone());
    if let Some(name) = env.storage().persistent().get::<_, String>(&key) {
        if let Some(value) = record(env, &name) {
            if value.owner == *user && is_live_or_grace(env, &value) {
                panic_with_error!(env, RegistryError::AlreadyHasUsername);
            }
        }
        env.storage().persistent().remove(&key);
    }
}

#[contractimpl]
impl UsernameRegistry {
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_token: Address,
        treasury: Address,
        automation: Address,
    ) {
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, RegistryError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(
            &DataKey::Config,
            &RegistryConfig {
                admin,
                fee_token,
                treasury,
                automation,
            },
        );
    }

    pub fn quote_registration(env: Env, username: String, lease_count: u32) -> NameQuote {
        quote(&env, &username, lease_count)
    }

    pub fn register_username(
        env: Env,
        user: Address,
        username: String,
        lease_count: u32,
    ) -> NameRecord {
        user.require_auth();
        clear_user_mapping_if_stale(&env, &user);
        let priced = quote(&env, &username, lease_count);
        if let Some(existing) = record(&env, &username) {
            if is_live_or_grace(&env, &existing) {
                panic_with_error!(&env, RegistryError::UsernameTaken);
            }
            env.storage()
                .persistent()
                .remove(&DataKey::UserName(existing.owner));
        }
        route_fee(&env, &user, priced.total_fee);
        let value = NameRecord {
            name: username.clone(),
            owner: user.clone(),
            registered_at: env.ledger().timestamp(),
            expires_at: priced.expires_at,
        };
        set_record(&env, &value);
        env.storage()
            .persistent()
            .set(&DataKey::UserName(user.clone()), &username);
        remember_name(&env, &username);
        env.events().publish(
            (Symbol::new(&env, "name_registered"), username),
            (user, value.expires_at, priced.total_fee),
        );
        value
    }

    pub fn renew_username(env: Env, user: Address, lease_count: u32) -> NameRecord {
        user.require_auth();
        validate_lease_count(&env, lease_count);
        let username: String = env
            .storage()
            .persistent()
            .get(&DataKey::UserName(user.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::AddressNotFound));
        let mut value = record(&env, &username)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::UsernameNotFound));
        if value.owner != user {
            panic_with_error!(&env, RegistryError::NotOwner);
        }
        if !is_live_or_grace(&env, &value) {
            panic_with_error!(&env, RegistryError::RenewalWindowClosed);
        }
        let fee = unit_price(&username) * i128::from(lease_count);
        route_fee(&env, &user, fee);
        value.expires_at =
            value.expires_at.max(env.ledger().timestamp()) + LEASE_SECONDS * u64::from(lease_count);
        set_record(&env, &value);
        value
    }

    pub fn change_username(
        env: Env,
        user: Address,
        new_username: String,
        lease_count: u32,
    ) -> NameRecord {
        user.require_auth();
        let old_username: String = env
            .storage()
            .persistent()
            .get(&DataKey::UserName(user.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::AddressNotFound));
        let old = record(&env, &old_username)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::UsernameNotFound));
        if old.owner != user || !is_live_or_grace(&env, &old) {
            panic_with_error!(&env, RegistryError::NotOwner);
        }
        let priced = quote(&env, &new_username, lease_count);
        if let Some(existing) = record(&env, &new_username) {
            if is_live_or_grace(&env, &existing) {
                panic_with_error!(&env, RegistryError::UsernameTaken);
            }
        }
        route_fee(&env, &user, priced.total_fee);
        env.storage()
            .persistent()
            .remove(&DataKey::Name(old_username));
        let value = NameRecord {
            name: new_username.clone(),
            owner: user.clone(),
            registered_at: env.ledger().timestamp(),
            expires_at: priced.expires_at,
        };
        set_record(&env, &value);
        env.storage()
            .persistent()
            .set(&DataKey::UserName(user), &new_username);
        remember_name(&env, &new_username);
        value
    }

    pub fn release_username(env: Env, user: Address) {
        user.require_auth();
        let key = DataKey::UserName(user.clone());
        let username: String = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::AddressNotFound));
        let value = record(&env, &username)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::UsernameNotFound));
        if value.owner != user {
            panic_with_error!(&env, RegistryError::NotOwner);
        }
        env.storage().persistent().remove(&key);
        env.storage().persistent().remove(&DataKey::Name(username));
    }

    pub fn get_address(env: Env, username: String) -> Address {
        let value = record(&env, &username)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::UsernameNotFound));
        if !is_live_or_grace(&env, &value) {
            panic_with_error!(&env, RegistryError::UsernameNotFound);
        }
        value.owner
    }

    pub fn get_username(env: Env, user: Address) -> String {
        let username: String = env
            .storage()
            .persistent()
            .get(&DataKey::UserName(user.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::AddressNotFound));
        let value = record(&env, &username)
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::AddressNotFound));
        if value.owner != user || !is_live_or_grace(&env, &value) {
            panic_with_error!(&env, RegistryError::AddressNotFound);
        }
        username
    }

    pub fn get_record(env: Env, username: String) -> Option<NameRecord> {
        record(&env, &username)
    }

    pub fn is_available(env: Env, username: String) -> bool {
        if !is_valid_username(&username) || is_reserved(&env, &username) {
            return false;
        }
        record(&env, &username)
            .map(|value| !is_live_or_grace(&env, &value))
            .unwrap_or(true)
    }

    pub fn list_names(env: Env, cursor: u32, limit: u32) -> Vec<NameRecord> {
        if limit == 0 || limit > 100 {
            panic_with_error!(&env, RegistryError::InvalidPagination);
        }
        let names: Vec<String> = env
            .storage()
            .persistent()
            .get(&DataKey::Names)
            .unwrap_or(Vec::new(&env));
        let mut output = Vec::new(&env);
        let end = (cursor + limit).min(names.len());
        let mut index = cursor;
        while index < end {
            if let Some(name) = names.get(index) {
                if let Some(value) = record(&env, &name) {
                    if is_live_or_grace(&env, &value) {
                        output.push_back(value);
                    }
                }
            }
            index += 1;
        }
        output
    }

    pub fn get_revenue(env: Env) -> (i128, i128) {
        (
            env.storage()
                .persistent()
                .get(&DataKey::TreasuryRevenue)
                .unwrap_or(0),
            env.storage()
                .persistent()
                .get(&DataKey::AutomationRevenue)
                .unwrap_or(0),
        )
    }

    pub fn get_config(env: Env) -> RegistryConfig {
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
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::NotAdmin));
        if expected != pending {
            panic_with_error!(&env, RegistryError::NotAdmin);
        }
        let mut cfg = config(&env);
        cfg.admin = pending;
        env.storage().instance().set(&DataKey::Config, &cfg);
        env.storage().instance().remove(&DataKey::PendingAdmin);
    }

    pub fn extend_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
    }
}

#[cfg(test)]
mod test;
