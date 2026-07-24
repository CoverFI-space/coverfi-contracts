#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, BytesN,
    Env, Symbol, Vec,
};

const RECEIPT_FEE: i128 = 1_000_000;
const MIN_BATCH_FEE: i128 = 10_000_000;
const PER_BATCH_RECEIPT_FEE: i128 = 20_000;
const DISPUTE_BOND: i128 = 10_000_000;
const INSTANCE_TTL_THRESHOLD: u32 = 30 * 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 120 * 17_280;

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
    pub owner: Address,
    pub receipt_hash: BytesN<32>,
    pub reference_hash: BytesN<32>,
    pub created_at: u64,
    pub status: ReceiptStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Batch {
    pub batch_id: u64,
    pub merchant: Address,
    pub merkle_root: BytesN<32>,
    pub receipt_count: u32,
    pub fee_paid: i128,
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
    pub result_hash: Option<BytesN<32>>,
    pub bond_amount: i128,
    pub bond_refunded: bool,
    pub status: DisputeStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TermsAcceptance {
    pub user: Address,
    pub terms_hash: BytesN<32>,
    pub accepted_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptConfig {
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
    NextReceiptId,
    NextBatchId,
    NextDisputeId,
    Receipt(u64),
    ReceiptHash(BytesN<32>),
    OwnerReceipts(Address),
    Viewers(u64),
    Batch(u64),
    Dispute(u64),
    Terms(Address),
    TreasuryRevenue,
    AutomationRevenue,
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
    DuplicateReceiptHash = 8,
    ViewerAlreadyExists = 9,
    ViewerNotFound = 10,
    BatchNotFound = 11,
    InvalidReceiptCount = 12,
    DisputeNotFound = 13,
    InvalidStatus = 14,
    TermsNotFound = 15,
}

fn config(env: &Env) -> ReceiptConfig {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .unwrap_or_else(|| panic_with_error!(env, ReceiptRegistryError::NotInitialized))
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != config(env).admin {
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

fn set_receipt(env: &Env, value: &Receipt) {
    env.storage()
        .persistent()
        .set(&DataKey::Receipt(value.receipt_id), value);
}

fn is_party(env: &Env, receipt_id: u64, user: &Address) -> bool {
    let value = receipt(env, receipt_id);
    *user == value.owner
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

fn route_fee(env: &Env, payer: &Address, amount: i128) {
    let cfg = config(env);
    let treasury_amount = amount;
    let automation_amount = 0_i128;
    let client = token::Client::new(env, &cfg.fee_token);
    client.transfer(payer, &cfg.treasury, &treasury_amount);
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

#[contractimpl]
impl ReceiptRegistry {
    pub fn initialize(
        env: Env,
        admin: Address,
        fee_token: Address,
        treasury: Address,
        automation: Address,
    ) {
        if env.storage().instance().has(&DataKey::Config) {
            panic_with_error!(&env, ReceiptRegistryError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(
            &DataKey::Config,
            &ReceiptConfig {
                admin,
                fee_token,
                treasury,
                automation,
            },
        );
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
        owner: Address,
        receipt_hash: BytesN<32>,
        reference_hash: BytesN<32>,
    ) -> u64 {
        owner.require_auth();
        let hash_key = DataKey::ReceiptHash(receipt_hash.clone());
        if env.storage().persistent().has(&hash_key) {
            panic_with_error!(&env, ReceiptRegistryError::DuplicateReceiptHash);
        }
        route_fee(&env, &owner, RECEIPT_FEE);
        let receipt_id = next_id(&env, DataKey::NextReceiptId);
        let value = Receipt {
            receipt_id,
            owner: owner.clone(),
            receipt_hash,
            reference_hash,
            created_at: env.ledger().timestamp(),
            status: ReceiptStatus::Active,
        };
        set_receipt(&env, &value);
        env.storage().persistent().set(&hash_key, &receipt_id);
        push_id(&env, DataKey::OwnerReceipts(owner), receipt_id);
        env.events().publish(
            (Symbol::new(&env, "receipt_anchored"), receipt_id),
            (value.receipt_hash, value.reference_hash, RECEIPT_FEE),
        );
        receipt_id
    }

    pub fn get_receipt_id_for_hash(env: Env, receipt_hash: BytesN<32>) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::ReceiptHash(receipt_hash))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::ReceiptNotFound))
    }

    pub fn get_receipt(env: Env, receipt_id: u64) -> Receipt {
        receipt(&env, receipt_id)
    }

    pub fn get_owner_receipts(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerReceipts(owner))
            .unwrap_or(Vec::new(&env))
    }

    pub fn verify_receipt(env: Env, receipt_id: u64, receipt_hash: BytesN<32>) -> bool {
        receipt(&env, receipt_id).receipt_hash == receipt_hash
    }

    pub fn is_receipt_party(env: Env, receipt_id: u64, user: Address) -> bool {
        is_party(&env, receipt_id, &user)
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

    pub fn quote_batch_fee(_env: Env, receipt_count: u32) -> i128 {
        if receipt_count == 0 {
            return 0;
        }
        (PER_BATCH_RECEIPT_FEE * i128::from(receipt_count)).max(MIN_BATCH_FEE)
    }

    pub fn anchor_batch(
        env: Env,
        merchant: Address,
        merkle_root: BytesN<32>,
        receipt_count: u32,
    ) -> u64 {
        merchant.require_auth();
        if receipt_count == 0 {
            panic_with_error!(&env, ReceiptRegistryError::InvalidReceiptCount);
        }
        let fee = Self::quote_batch_fee(env.clone(), receipt_count);
        route_fee(&env, &merchant, fee);
        let batch_id = next_id(&env, DataKey::NextBatchId);
        let value = Batch {
            batch_id,
            merchant,
            merkle_root,
            receipt_count,
            fee_paid: fee,
            created_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Batch(batch_id), &value);
        batch_id
    }

    pub fn verify_batch(env: Env, batch_id: u64, merkle_root: BytesN<32>) -> bool {
        Self::get_batch(env, batch_id).merkle_root == merkle_root
    }

    pub fn get_batch(env: Env, batch_id: u64) -> Batch {
        env.storage()
            .persistent()
            .get(&DataKey::Batch(batch_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::BatchNotFound))
    }

    pub fn open_dispute(env: Env, user: Address, receipt_id: u64, reason_hash: BytesN<32>) -> u64 {
        require_party(&env, receipt_id, &user);
        let mut value = receipt(&env, receipt_id);
        if value.status == ReceiptStatus::Disputed {
            panic_with_error!(&env, ReceiptRegistryError::AlreadyDisputed);
        }
        token::Client::new(&env, &config(&env).fee_token).transfer(
            &user,
            &env.current_contract_address(),
            &DISPUTE_BOND,
        );
        value.status = ReceiptStatus::Disputed;
        set_receipt(&env, &value);
        let dispute_id = next_id(&env, DataKey::NextDisputeId);
        env.storage().persistent().set(
            &DataKey::Dispute(dispute_id),
            &Dispute {
                dispute_id,
                receipt_id,
                opener: user,
                reason_hash,
                response_hash: None,
                result_hash: None,
                bond_amount: DISPUTE_BOND,
                bond_refunded: false,
                status: DisputeStatus::Open,
            },
        );
        dispute_id
    }

    pub fn respond_dispute(env: Env, user: Address, dispute_id: u64, response_hash: BytesN<32>) {
        let mut value = Self::get_dispute(env.clone(), dispute_id);
        if value.status != DisputeStatus::Open {
            panic_with_error!(&env, ReceiptRegistryError::InvalidStatus);
        }
        require_party(&env, value.receipt_id, &user);
        if value.opener == user {
            panic_with_error!(&env, ReceiptRegistryError::NotAuthorized);
        }
        value.response_hash = Some(response_hash);
        value.status = DisputeStatus::Responded;
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &value);
    }

    pub fn resolve_dispute(env: Env, admin: Address, dispute_id: u64, result_hash: BytesN<32>) {
        require_admin(&env, &admin);
        let mut value = Self::get_dispute(env.clone(), dispute_id);
        if value.status == DisputeStatus::Resolved {
            panic_with_error!(&env, ReceiptRegistryError::InvalidStatus);
        }
        token::Client::new(&env, &config(&env).fee_token).transfer(
            &env.current_contract_address(),
            &value.opener,
            &value.bond_amount,
        );
        value.result_hash = Some(result_hash);
        value.bond_refunded = true;
        value.status = DisputeStatus::Resolved;
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(dispute_id), &value);
        let mut receipt_value = receipt(&env, value.receipt_id);
        receipt_value.status = ReceiptStatus::Active;
        set_receipt(&env, &receipt_value);
    }

    pub fn get_dispute(env: Env, dispute_id: u64) -> Dispute {
        env.storage()
            .persistent()
            .get(&DataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::DisputeNotFound))
    }

    pub fn record_terms_acceptance(
        env: Env,
        user: Address,
        terms_hash: BytesN<32>,
    ) -> TermsAcceptance {
        user.require_auth();
        let value = TermsAcceptance {
            user: user.clone(),
            terms_hash,
            accepted_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Terms(user), &value);
        value
    }

    pub fn get_terms_acceptance(env: Env, user: Address) -> TermsAcceptance {
        env.storage()
            .persistent()
            .get(&DataKey::Terms(user))
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::TermsNotFound))
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

    pub fn get_config(env: Env) -> ReceiptConfig {
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
            .unwrap_or_else(|| panic_with_error!(&env, ReceiptRegistryError::NotAdmin));
        if expected != pending {
            panic_with_error!(&env, ReceiptRegistryError::NotAdmin);
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
