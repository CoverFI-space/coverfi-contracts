#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, String,
};

#[contract]
pub struct UsernameRegistry;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
enum DataKey {
    UsernameToAddress(String),
    AddressToUsername(Address),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RegistryError {
    UsernameTaken = 1,
    UsernameNotFound = 2,
    AddressNotFound = 3,
    InvalidUsername = 4,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Validate 3 to 24 chars, alphanumeric + underscore
fn is_valid_username(_env: &Env, username: &String) -> bool {
    let len = username.len() as usize;
    if len < 3 || len > 24 {
        return false;
    }
    
    let mut bytes = [0u8; 24];
    username.copy_into_slice(&mut bytes[..len]);
    
    for i in 0..len {
        let b = bytes[i];
        let is_lower = b >= b'a' && b <= b'z';
        let is_upper = b >= b'A' && b <= b'Z';
        let is_digit = b >= b'0' && b <= b'9';
        let is_underscore = b == b'_';
        if !(is_lower || is_upper || is_digit || is_underscore) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Contract implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl UsernameRegistry {
    /// Register a new username for the caller.
    pub fn register_username(env: Env, user: Address, username: String) {
        user.require_auth();

        if !is_valid_username(&env, &username) {
            panic_with_error!(&env, RegistryError::InvalidUsername);
        }

        let u2a_key = DataKey::UsernameToAddress(username.clone());
        if env.storage().persistent().has(&u2a_key) {
            panic_with_error!(&env, RegistryError::UsernameTaken);
        }

        let a2u_key = DataKey::AddressToUsername(user.clone());
        
        // If they already have a username, we should remove the old mapping.
        if let Some(old_username) = env.storage().persistent().get::<_, String>(&a2u_key) {
            let old_u2a = DataKey::UsernameToAddress(old_username);
            env.storage().persistent().remove(&old_u2a);
        }

        env.storage().persistent().set(&u2a_key, &user);
        env.storage().persistent().set(&a2u_key, &username);
    }

    /// Update username for the caller.
    pub fn update_username(env: Env, user: Address, new_username: String) {
        // Just reuse register_username as it handles the logic
        Self::register_username(env, user, new_username);
    }

    /// Retrieve the address for a given username.
    pub fn get_address(env: Env, username: String) -> Address {
        env.storage()
            .persistent()
            .get(&DataKey::UsernameToAddress(username))
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::UsernameNotFound))
    }

    /// Retrieve the username for a given address.
    pub fn get_username(env: Env, user: Address) -> String {
        env.storage()
            .persistent()
            .get(&DataKey::AddressToUsername(user))
            .unwrap_or_else(|| panic_with_error!(&env, RegistryError::AddressNotFound))
    }

    /// Check if a username is available.
    pub fn is_available(env: Env, username: String) -> bool {
        if !is_valid_username(&env, &username) {
            return false;
        }
        !env.storage()
            .persistent()
            .has(&DataKey::UsernameToAddress(username))
    }
}

#[cfg(test)]
mod test;
