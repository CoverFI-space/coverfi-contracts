#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env,
    String, Vec,
};

#[contract]
pub struct PaymentReceiptRegistry;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptStatus {
    Active,
    Disputed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub receipt_id: u64,
    pub sender: Address,
    pub receiver: Address,
    pub payment_tx_hash: String,
    pub receipt_hash: BytesN<32>,
    pub encrypted_receipt_uri: String,
    pub created_at: u64,
    pub status: ReceiptStatus,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    NextReceiptId,
    Receipt(u64),
    SentReceipts(Address),
    ReceivedReceipts(Address),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ReceiptError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    ReceiptNotFound = 4,
    NotAuthorized = 5,
    AlreadyDisputed = 6,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn next_receipt_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextReceiptId)
        .unwrap_or(1_u64);
    env.storage()
        .instance()
        .set(&DataKey::NextReceiptId, &(id + 1));
    id
}

fn receipt(env: &Env, receipt_id: u64) -> Receipt {
    env.storage()
        .persistent()
        .get(&DataKey::Receipt(receipt_id))
        .unwrap_or_else(|| panic_with_error!(env, ReceiptError::ReceiptNotFound))
}

fn set_receipt(env: &Env, r: &Receipt) {
    env.storage()
        .persistent()
        .set(&DataKey::Receipt(r.receipt_id), r);
}

fn push_sent(env: &Env, sender: &Address, receipt_id: u64) {
    let key = DataKey::SentReceipts(sender.clone());
    let mut list: Vec<u64> = env.storage().persistent().get(&key).unwrap_or(Vec::new(env));
    list.push_back(receipt_id);
    env.storage().persistent().set(&key, &list);
}

fn push_received(env: &Env, receiver: &Address, receipt_id: u64) {
    let key = DataKey::ReceivedReceipts(receiver.clone());
    let mut list: Vec<u64> = env.storage().persistent().get(&key).unwrap_or(Vec::new(env));
    list.push_back(receipt_id);
    env.storage().persistent().set(&key, &list);
}

// ---------------------------------------------------------------------------
// Contract implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl PaymentReceiptRegistry {
    /// Initialise the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, ReceiptError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextReceiptId, &1_u64);
    }

    /// Create a new receipt proof on-chain. Sender must authorise.
    pub fn create_receipt(
        env: Env,
        sender: Address,
        receiver: Address,
        payment_tx_hash: String,
        receipt_hash: BytesN<32>,
        encrypted_receipt_uri: String,
    ) -> u64 {
        // Require the sender to have signed this call.
        sender.require_auth();

        let id = next_receipt_id(&env);
        let r = Receipt {
            receipt_id: id,
            sender: sender.clone(),
            receiver: receiver.clone(),
            payment_tx_hash,
            receipt_hash,
            encrypted_receipt_uri,
            created_at: env.ledger().timestamp(),
            status: ReceiptStatus::Active,
        };

        set_receipt(&env, &r);
        push_sent(&env, &sender, id);
        push_received(&env, &receiver, id);

        id
    }

    /// Retrieve a receipt by its ID.
    pub fn get_receipt(env: Env, receipt_id: u64) -> Receipt {
        receipt(&env, receipt_id)
    }

    /// Return a list of receipt IDs sent by the given address.
    pub fn get_sent_receipts(env: Env, sender: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::SentReceipts(sender))
            .unwrap_or(Vec::new(&env))
    }

    /// Return a list of receipt IDs received by the given address.
    pub fn get_received_receipts(env: Env, receiver: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::ReceivedReceipts(receiver))
            .unwrap_or(Vec::new(&env))
    }

    /// Verify that a given hash matches the stored receipt hash.
    pub fn verify_receipt(env: Env, receipt_id: u64, receipt_hash: BytesN<32>) -> bool {
        let r = receipt(&env, receipt_id);
        r.receipt_hash == receipt_hash
    }

    /// Mark a receipt as disputed. Caller must be sender or receiver.
    pub fn mark_disputed(env: Env, user: Address, receipt_id: u64) {
        user.require_auth();

        let mut r = receipt(&env, receipt_id);

        if r.status == ReceiptStatus::Disputed {
            panic_with_error!(&env, ReceiptError::AlreadyDisputed);
        }

        if user != r.sender && user != r.receiver {
            panic_with_error!(&env, ReceiptError::NotAuthorized);
        }

        r.status = ReceiptStatus::Disputed;
        set_receipt(&env, &r);
    }
}

#[cfg(test)]
mod test;
