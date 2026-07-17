#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env,
    String, Vec,
};

#[contract]
pub struct ReceiptRegistry;

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Batch {
    pub batch_id: u64,
    pub merkle_root: BytesN<32>,
    pub receipt_count: u32,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    Open,
    Responded,
    Resolved,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dispute {
    pub dispute_id: u64,
    pub receipt_id: u64,
    pub opener: Address,
    pub reason_hash: BytesN<32>,
    pub response_hash: Option<BytesN<32>>,
    pub result: Option<String>,
    pub status: DisputeStatus,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    NextReceiptId,
    NextBatchId,
    NextDisputeId,
    Receipt(u64),
    PaymentReceipt(String),
    SentReceipts(Address),
    ReceivedReceipts(Address),
    Viewers(u64),
    Batch(u64),
    Dispute(u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ReceiptRegistryError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    ReceiptNotFound = 4,
    NotAuthorized = 5,
    AlreadyDisputed = 6,
    InvalidReceiptMetadata = 7,
    DuplicatePaymentHash = 8,
    ViewerAlreadyExists = 9,
    ViewerNotFound = 10,
    BatchNotFound = 11,
    InvalidReceiptCount = 12,
    DisputeNotFound = 13,
    InvalidStatus = 14,
}

fn admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, ReceiptRegistryError::NotInitialized))
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != admin(env) {
        panic_with_error!(env, ReceiptRegistryError::NotAdmin);
    }
}

fn next_id(env: &Env, key: DataKey) -> u64 {
    let id: u64 = env.storage().instance().get(&key).unwrap_or(1_u64);
    env.storage().instance().set(&key, &(id + 1));
    id
}

fn receipt(env: &Env, receipt_id: u64) -> Receipt {
    env.storage()
        .persistent()
        .get(&DataKey::Receipt(receipt_id))
        .unwrap_or_else(|| panic_with_error!(env, ReceiptRegistryError::ReceiptNotFound))
}

fn set_receipt(env: &Env, record: &Receipt) {
    env.storage()
        .persistent()
        .set(&DataKey::Receipt(record.receipt_id), record);
}

fn is_party(env: &Env, receipt_id: u64, user: &Address) -> bool {
    let record = receipt(env, receipt_id);
    *user == record.sender || *user == record.receiver
}

fn require_party(env: &Env, receipt_id: u64, user: &Address) {
    user.require_auth();
    if !is_party(env, receipt_id, user) {
        panic_with_error!(env, ReceiptRegistryError::NotAuthorized);
    }
}

fn push_id(env: &Env, key: DataKey, receipt_id: u64) {
    let mut list: Vec<u64> = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or(Vec::new(env));
    list.push_back(receipt_id);
    env.storage().persistent().set(&key, &list);
}

fn validate_receipt_metadata(env: &Env, payment_tx_hash: &String, encrypted_receipt_uri: &String) {
    let tx_len = payment_tx_hash.len();
    let uri_len = encrypted_receipt_uri.len();
    if tx_len == 0 || tx_len > 128 || uri_len == 0 || uri_len > 256 {
        panic_with_error!(env, ReceiptRegistryError::InvalidReceiptMetadata);
    }
}

#[contractimpl]
impl ReceiptRegistry {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, ReceiptRegistryError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::NextReceiptId, &1_u64);
        env.storage().instance().set(&DataKey::NextBatchId, &1_u64);
        env.storage()
            .instance()
            .set(&DataKey::NextDisputeId, &1_u64);
    }

    pub fn create_receipt(
        env: Env,
        sender: Address,
        receiver: Address,
        payment_tx_hash: String,
        receipt_hash: BytesN<32>,
        encrypted_receipt_uri: String,
    ) -> u64 {
        sender.require_auth();
        validate_receipt_metadata(&env, &payment_tx_hash, &encrypted_receipt_uri);
        let payment_key = DataKey::PaymentReceipt(payment_tx_hash.clone());
        if env.storage().persistent().has(&payment_key) {
            panic_with_error!(&env, ReceiptRegistryError::DuplicatePaymentHash);
        }

        let receipt_id = next_id(&env, DataKey::NextReceiptId);
        let record = Receipt {
            receipt_id,
            sender: sender.clone(),
            receiver: receiver.clone(),
            payment_tx_hash,
            receipt_hash,
            encrypted_receipt_uri,
            created_at: env.ledger().timestamp(),
            status: ReceiptStatus::Active,
        };

        set_receipt(&env, &record);
        env.storage().persistent().set(&payment_key, &receipt_id);
        push_id(&env, DataKey::SentReceipts(sender), receipt_id);
        push_id(&env, DataKey::ReceivedReceipts(receiver), receipt_id);
        receipt_id
    }

    pub fn get_receipt_id_for_payment_hash(env: Env, payment_tx_hash: String) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::PaymentReceipt(payment_tx_hash))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::ReceiptNotFound))
    }

    pub fn get_receipt(env: Env, receipt_id: u64) -> Receipt {
        receipt(&env, receipt_id)
    }

    pub fn get_sent_receipts(env: Env, sender: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::SentReceipts(sender))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_received_receipts(env: Env, receiver: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::ReceivedReceipts(receiver))
            .unwrap_or(Vec::new(&env))
    }

    pub fn verify_receipt(env: Env, receipt_id: u64, receipt_hash: BytesN<32>) -> bool {
        receipt(&env, receipt_id).receipt_hash == receipt_hash
    }

    pub fn is_receipt_party(env: Env, receipt_id: u64, user: Address) -> bool {
        is_party(&env, receipt_id, &user)
    }

    pub fn mark_disputed(env: Env, user: Address, receipt_id: u64) {
        require_party(&env, receipt_id, &user);
        let mut record = receipt(&env, receipt_id);
        if record.status == ReceiptStatus::Disputed {
            panic_with_error!(&env, ReceiptRegistryError::AlreadyDisputed);
        }
        record.status = ReceiptStatus::Disputed;
        set_receipt(&env, &record);
    }

    pub fn grant_access(env: Env, caller: Address, receipt_id: u64, viewer: Address) {
        require_party(&env, receipt_id, &caller);
        let key = DataKey::Viewers(receipt_id);
        let mut viewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env));
        if viewers.contains(&viewer) {
            panic_with_error!(&env, ReceiptRegistryError::ViewerAlreadyExists);
        }
        viewers.push_back(viewer);
        env.storage().persistent().set(&key, &viewers);
    }

    pub fn revoke_access(env: Env, caller: Address, receipt_id: u64, viewer: Address) {
        require_party(&env, receipt_id, &caller);
        let key = DataKey::Viewers(receipt_id);
        let mut viewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(&env));
        if let Some(index) = viewers.first_index_of(&viewer) {
            viewers.remove(index);
            env.storage().persistent().set(&key, &viewers);
            return;
        }
        panic_with_error!(&env, ReceiptRegistryError::ViewerNotFound);
    }

    pub fn has_access(env: Env, receipt_id: u64, viewer: Address) -> bool {
        if is_party(&env, receipt_id, &viewer) {
            return true;
        }
        let viewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Viewers(receipt_id))
            .unwrap_or(Vec::new(&env));
        viewers.contains(&viewer)
    }

    pub fn get_viewers(env: Env, receipt_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::Viewers(receipt_id))
            .unwrap_or(Vec::new(&env))
    }

    pub fn anchor_batch(
        env: Env,
        admin: Address,
        merkle_root: BytesN<32>,
        receipt_count: u32,
    ) -> u64 {
        require_admin(&env, &admin);
        if receipt_count == 0 {
            panic_with_error!(&env, ReceiptRegistryError::InvalidReceiptCount);
        }

        let batch_id = next_id(&env, DataKey::NextBatchId);
        let batch = Batch {
            batch_id,
            merkle_root,
            receipt_count,
            created_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Batch(batch_id), &batch);
        batch_id
    }

    pub fn verify_batch(env: Env, batch_id: u64, merkle_root: BytesN<32>) -> bool {
        let batch: Batch = env
            .storage()
            .persistent()
            .get(&DataKey::Batch(batch_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::BatchNotFound));
        batch.merkle_root == merkle_root
    }

    pub fn get_batch(env: Env, batch_id: u64) -> Batch {
        env.storage()
            .persistent()
            .get(&DataKey::Batch(batch_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::BatchNotFound))
    }

    pub fn open_dispute(env: Env, user: Address, receipt_id: u64, reason_hash: BytesN<32>) -> u64 {
        require_party(&env, receipt_id, &user);
        let mut record = receipt(&env, receipt_id);
        if record.status == ReceiptStatus::Disputed {
            panic_with_error!(&env, ReceiptRegistryError::AlreadyDisputed);
        }
        record.status = ReceiptStatus::Disputed;
        set_receipt(&env, &record);

        let dispute_id = next_id(&env, DataKey::NextDisputeId);
        let dispute = Dispute {
            dispute_id,
            receipt_id,
            opener: user,
            reason_hash,
            response_hash: None,
            result: None,
            status: DisputeStatus::Open,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &dispute);
        dispute_id
    }

    pub fn respond_dispute(env: Env, user: Address, dispute_id: u64, response_hash: BytesN<32>) {
        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::DisputeNotFound));
        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, ReceiptRegistryError::InvalidStatus);
        }
        require_party(&env, dispute.receipt_id, &user);
        if dispute.opener == user {
            panic_with_error!(&env, ReceiptRegistryError::NotAuthorized);
        }

        dispute.response_hash = Some(response_hash);
        dispute.status = DisputeStatus::Responded;
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &dispute);
    }

    pub fn resolve_dispute(env: Env, admin: Address, dispute_id: u64, result: String) {
        require_admin(&env, &admin);
        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::DisputeNotFound));
        if dispute.status == DisputeStatus::Resolved {
            panic_with_error!(&env, ReceiptRegistryError::InvalidStatus);
        }

        dispute.result = Some(result);
        dispute.status = DisputeStatus::Resolved;
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &dispute);
    }

    pub fn get_dispute(env: Env, dispute_id: u64) -> Dispute {
        env.storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::DisputeNotFound))
    }
}
