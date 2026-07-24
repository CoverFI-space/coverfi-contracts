#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Symbol,
};

const BPS: i128 = 10_000;
const UNDERWRITER_BPS: i128 = 7_800;
const DEFAULT_PROTOCOL_BPS: i128 = 1_200;
const MIN_PROTOCOL_BPS: i128 = 1_000;
const MAX_PROTOCOL_BPS: i128 = 1_500;
const SAFETY_BPS: i128 = 700;
const PARTNER_BPS: i128 = 2_000;
const MAX_UTILIZATION_BPS: i128 = 9_000;
const MAX_CONCENTRATION_BPS: i128 = 7_500;
const WITHDRAW_DELAY: u64 = 7 * 86_400;
const INSTANCE_TTL_THRESHOLD: u32 = 30 * 17_280;
const INSTANCE_TTL_EXTEND_TO: u32 = 120 * 17_280;

#[contract]
pub struct ReserveVault;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolState {
    pub token: Address,
    pub total_assets: i128,
    pub total_shares: i128,
    pub locked_liabilities: i128,
    pub reserved_claims: i128,
    pub unearned_premiums: i128,
    pub safety_balance: i128,
    pub automation_balance: i128,
    pub cumulative_protocol_revenue: i128,
    pub cumulative_underwriting: i128,
    pub cumulative_claims: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalRequest {
    pub id: u64,
    pub owner: Address,
    pub token: Address,
    pub shares: i128,
    pub executable_at: u64,
    pub completed: bool,
    pub cancelled: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimRecord {
    pub position_id: u64,
    pub owner: Address,
    pub token: Address,
    pub amount: i128,
    pub withdrawn: bool,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    PendingAdmin,
    Engine,
    Oracle,
    Treasury,
    NextWithdrawalId,
    Pool(Address),
    Shares(Address, Address),
    QueuedShares(Address, Address),
    Withdrawal(u64),
    PositionLock(u64),
    PositionToken(u64),
    PositionMarket(u64),
    MarketLocked(Address, Address),
    PositionUnearned(u64),
    Claim(u64),
    PartnerApproved(Address),
    ProtocolFeeBps,
    RewardPaid(Symbol, u64),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ReserveError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    InvalidAmount = 4,
    InsufficientLiquidity = 5,
    InsufficientShares = 6,
    Slippage = 7,
    RequestNotReady = 8,
    RequestClosed = 9,
    NotOwner = 10,
    TokenMismatch = 11,
    PositionNotLocked = 12,
    ClaimNotFound = 13,
    AlreadyWithdrawn = 14,
    RewardAlreadyPaid = 15,
    UnsafeUtilization = 16,
    ConcentrationLimit = 17,
    PositionAlreadyLocked = 18,
    InvalidProtocolFee = 19,
}

fn read_address(env: &Env, key: DataKey) -> Address {
    env.storage()
        .instance()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, ReserveError::NotInitialized))
}

fn admin(env: &Env) -> Address {
    read_address(env, DataKey::Admin)
}

fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    if *caller != admin(env) {
        panic_with_error!(env, ReserveError::NotAuthorized);
    }
}

fn require_engine(env: &Env, caller: &Address) {
    if *caller != read_address(env, DataKey::Engine) {
        panic_with_error!(env, ReserveError::NotAuthorized);
    }
}

fn require_reward_caller(env: &Env, caller: &Address) {
    let engine = read_address(env, DataKey::Engine);
    let oracle = read_address(env, DataKey::Oracle);
    if *caller != engine && *caller != oracle {
        panic_with_error!(env, ReserveError::NotAuthorized);
    }
}

fn empty_pool(token: Address) -> PoolState {
    PoolState {
        token,
        total_assets: 0,
        total_shares: 0,
        locked_liabilities: 0,
        reserved_claims: 0,
        unearned_premiums: 0,
        safety_balance: 0,
        automation_balance: 0,
        cumulative_protocol_revenue: 0,
        cumulative_underwriting: 0,
        cumulative_claims: 0,
    }
}

fn pool(env: &Env, token: &Address) -> PoolState {
    env.storage()
        .persistent()
        .get(&DataKey::Pool(token.clone()))
        .unwrap_or_else(|| empty_pool(token.clone()))
}

fn set_pool(env: &Env, state: &PoolState) {
    env.storage()
        .persistent()
        .set(&DataKey::Pool(state.token.clone()), state);
}

fn read_amount(env: &Env, key: DataKey) -> i128 {
    env.storage().persistent().get(&key).unwrap_or(0_i128)
}

fn provider_nav(state: &PoolState) -> i128 {
    state.total_assets
        - state.safety_balance
        - state.automation_balance
        - state.unearned_premiums
        - state.reserved_claims
}

fn liquid_assets(state: &PoolState) -> i128 {
    state.total_assets
        - state.locked_liabilities
        - state.reserved_claims
        - state.safety_balance
        - state.automation_balance
        - state.unearned_premiums
}

fn checked_positive(env: &Env, amount: i128) {
    if amount <= 0 {
        panic_with_error!(env, ReserveError::InvalidAmount);
    }
}

fn protocol_fee_bps(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::ProtocolFeeBps)
        .unwrap_or(DEFAULT_PROTOCOL_BPS)
}

fn validate_protocol_fee_bps(env: &Env, value: i128) {
    if !(MIN_PROTOCOL_BPS..=MAX_PROTOCOL_BPS).contains(&value)
        || UNDERWRITER_BPS + SAFETY_BPS + value > BPS
    {
        panic_with_error!(env, ReserveError::InvalidProtocolFee);
    }
}

#[contractimpl]
impl ReserveVault {
    pub fn initialize(env: Env, admin: Address, treasury: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, ReserveError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage()
            .instance()
            .set(&DataKey::ProtocolFeeBps, &DEFAULT_PROTOCOL_BPS);
        env.storage()
            .instance()
            .set(&DataKey::NextWithdrawalId, &1_u64);
    }

    pub fn configure_protocol(env: Env, admin: Address, engine: Address, oracle: Address) {
        require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Engine, &engine);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
    }

    pub fn propose_admin(env: Env, admin: Address, pending_admin: Address) {
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &pending_admin);
    }

    pub fn accept_admin(env: Env, pending_admin: Address) {
        pending_admin.require_auth();
        let expected: Address = read_address(&env, DataKey::PendingAdmin);
        if pending_admin != expected {
            panic_with_error!(&env, ReserveError::NotAuthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::Admin, &pending_admin);
        env.storage().instance().remove(&DataKey::PendingAdmin);
    }

    pub fn set_partner(env: Env, admin: Address, partner: Address, approved: bool) {
        require_admin(&env, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::PartnerApproved(partner), &approved);
    }

    pub fn set_treasury(env: Env, admin: Address, treasury: Address) {
        require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.events()
            .publish((Symbol::new(&env, "treasury_set"),), treasury);
    }

    pub fn get_treasury(env: Env) -> Address {
        read_address(&env, DataKey::Treasury)
    }

    pub fn set_protocol_fee_bps(env: Env, admin: Address, fee_bps: i128) {
        require_admin(&env, &admin);
        validate_protocol_fee_bps(&env, fee_bps);
        env.storage()
            .instance()
            .set(&DataKey::ProtocolFeeBps, &fee_bps);
        env.events()
            .publish((Symbol::new(&env, "protocol_fee_set"),), fee_bps);
    }

    pub fn get_protocol_fee_bps(env: Env) -> i128 {
        protocol_fee_bps(&env)
    }

    pub fn deposit_reserve(
        env: Env,
        provider: Address,
        token: Address,
        amount: i128,
        min_shares: i128,
    ) -> i128 {
        provider.require_auth();
        checked_positive(&env, amount);
        let mut state = pool(&env, &token);
        let nav = provider_nav(&state);
        let shares = if state.total_shares == 0 || nav <= 0 {
            amount
        } else {
            amount * state.total_shares / nav
        };
        if shares <= 0 || shares < min_shares {
            panic_with_error!(&env, ReserveError::Slippage);
        }
        token::Client::new(&env, &token).transfer(
            &provider,
            &env.current_contract_address(),
            &amount,
        );
        state.total_assets += amount;
        state.total_shares += shares;
        set_pool(&env, &state);
        let key = DataKey::Shares(provider.clone(), token.clone());
        env.storage()
            .persistent()
            .set(&key, &(read_amount(&env, key.clone()) + shares));
        env.events().publish(
            (Symbol::new(&env, "reserve_deposited"), provider),
            (token, amount, shares),
        );
        shares
    }

    pub fn fund_automation(env: Env, funder: Address, token: Address, amount: i128) {
        funder.require_auth();
        checked_positive(&env, amount);
        token::Client::new(&env, &token).transfer(
            &funder,
            &env.current_contract_address(),
            &amount,
        );
        let mut state = pool(&env, &token);
        state.total_assets += amount;
        state.automation_balance += amount;
        set_pool(&env, &state);
    }

    pub fn collect_premium_from(
        env: Env,
        engine: Address,
        user: Address,
        token: Address,
        risk_premium: i128,
        automation_fee: i128,
        position_id: u64,
        partner: Option<Address>,
    ) {
        require_engine(&env, &engine);
        checked_positive(&env, risk_premium);
        if automation_fee < 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }
        let total = risk_premium + automation_fee;
        token::Client::new(&env, &token).transfer(&user, &env.current_contract_address(), &total);

        let underwriting = risk_premium * UNDERWRITER_BPS / BPS;
        let protocol_bps = protocol_fee_bps(&env);
        validate_protocol_fee_bps(&env, protocol_bps);
        let mut protocol = risk_premium * protocol_bps / BPS;
        let safety = risk_premium * SAFETY_BPS / BPS;
        let automation = risk_premium - underwriting - protocol - safety + automation_fee;
        let mut partner_amount = 0_i128;
        if let Some(partner_address) = partner {
            let approved: bool = env
                .storage()
                .persistent()
                .get(&DataKey::PartnerApproved(partner_address.clone()))
                .unwrap_or(false);
            if approved {
                partner_amount = protocol * PARTNER_BPS / BPS;
                token::Client::new(&env, &token).transfer(
                    &env.current_contract_address(),
                    &partner_address,
                    &partner_amount,
                );
                protocol -= partner_amount;
            }
        }
        let treasury = read_address(&env, DataKey::Treasury);
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &treasury,
            &protocol,
        );

        let mut state = pool(&env, &token);
        state.total_assets += underwriting + safety + automation;
        state.unearned_premiums += underwriting;
        state.safety_balance += safety;
        state.automation_balance += automation;
        state.cumulative_protocol_revenue += protocol + partner_amount;
        state.cumulative_underwriting += underwriting;
        set_pool(&env, &state);
        env.storage()
            .persistent()
            .set(&DataKey::PositionUnearned(position_id), &underwriting);
        env.events().publish(
            (Symbol::new(&env, "premium_routed"), position_id),
            (underwriting, protocol, partner_amount, safety, automation),
        );
    }

    pub fn lock_payout_capacity(
        env: Env,
        engine: Address,
        protected_asset: Address,
        payout_token: Address,
        position_id: u64,
        amount: i128,
    ) {
        require_engine(&env, &engine);
        checked_positive(&env, amount);
        if read_amount(&env, DataKey::PositionLock(position_id)) != 0 {
            panic_with_error!(&env, ReserveError::PositionAlreadyLocked);
        }
        let mut state = pool(&env, &payout_token);
        if liquid_assets(&state) < amount {
            panic_with_error!(&env, ReserveError::InsufficientLiquidity);
        }
        let nav = provider_nav(&state);
        if nav <= 0 || (state.locked_liabilities + amount) * BPS / nav >= MAX_UTILIZATION_BPS {
            panic_with_error!(&env, ReserveError::UnsafeUtilization);
        }
        let market_key = DataKey::MarketLocked(protected_asset.clone(), payout_token.clone());
        let projected_market = read_amount(&env, market_key.clone()) + amount;
        if projected_market * BPS / nav > MAX_CONCENTRATION_BPS {
            panic_with_error!(&env, ReserveError::ConcentrationLimit);
        }
        state.locked_liabilities += amount;
        set_pool(&env, &state);
        env.storage()
            .persistent()
            .set(&DataKey::PositionLock(position_id), &amount);
        env.storage()
            .persistent()
            .set(&DataKey::PositionToken(position_id), &payout_token);
        env.storage()
            .persistent()
            .set(&DataKey::PositionMarket(position_id), &protected_asset);
        env.storage().persistent().set(
            &market_key,
            &(read_amount(&env, market_key.clone()) + amount),
        );
    }

    pub fn settle_position(
        env: Env,
        engine: Address,
        owner: Address,
        position_id: u64,
        payout: i128,
    ) {
        require_engine(&env, &engine);
        if payout < 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }
        let lock = read_amount(&env, DataKey::PositionLock(position_id));
        if lock <= 0 || payout > lock {
            panic_with_error!(&env, ReserveError::PositionNotLocked);
        }
        let token: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PositionToken(position_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReserveError::PositionNotLocked));
        let market: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PositionMarket(position_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReserveError::PositionNotLocked));
        let mut state = pool(&env, &token);
        state.locked_liabilities -= lock;
        state.reserved_claims += payout;
        let unearned = read_amount(&env, DataKey::PositionUnearned(position_id));
        state.unearned_premiums -= unearned;
        state.cumulative_claims += payout;
        set_pool(&env, &state);
        let market_key = DataKey::MarketLocked(market, token.clone());
        env.storage()
            .persistent()
            .set(&market_key, &(read_amount(&env, market_key.clone()) - lock));
        env.storage()
            .persistent()
            .set(&DataKey::PositionLock(position_id), &0_i128);
        env.storage()
            .persistent()
            .set(&DataKey::PositionUnearned(position_id), &0_i128);
        if payout > 0 {
            env.storage().persistent().set(
                &DataKey::Claim(position_id),
                &ClaimRecord {
                    position_id,
                    owner,
                    token,
                    amount: payout,
                    withdrawn: false,
                },
            );
        }
    }

    pub fn claim_payout(env: Env, engine: Address, owner: Address, position_id: u64) -> i128 {
        require_engine(&env, &engine);
        let mut claim: ClaimRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Claim(position_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReserveError::ClaimNotFound));
        if claim.owner != owner {
            panic_with_error!(&env, ReserveError::NotOwner);
        }
        if claim.withdrawn {
            panic_with_error!(&env, ReserveError::AlreadyWithdrawn);
        }
        claim.withdrawn = true;
        env.storage()
            .persistent()
            .set(&DataKey::Claim(position_id), &claim);
        let mut state = pool(&env, &claim.token);
        state.total_assets -= claim.amount;
        state.reserved_claims -= claim.amount;
        set_pool(&env, &state);
        token::Client::new(&env, &claim.token).transfer(
            &env.current_contract_address(),
            &owner,
            &claim.amount,
        );
        claim.amount
    }

    pub fn pay_keeper(
        env: Env,
        caller: Address,
        token: Address,
        keeper: Address,
        action: Symbol,
        action_id: u64,
        amount: i128,
    ) -> bool {
        require_reward_caller(&env, &caller);
        checked_positive(&env, amount);
        let reward_key = DataKey::RewardPaid(action.clone(), action_id);
        if env.storage().persistent().has(&reward_key) {
            panic_with_error!(&env, ReserveError::RewardAlreadyPaid);
        }
        let mut state = pool(&env, &token);
        if state.automation_balance < amount {
            return false;
        }
        state.automation_balance -= amount;
        state.total_assets -= amount;
        set_pool(&env, &state);
        env.storage().persistent().set(&reward_key, &true);
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &keeper,
            &amount,
        );
        true
    }

    pub fn request_withdraw(env: Env, provider: Address, token: Address, shares: i128) -> u64 {
        provider.require_auth();
        checked_positive(&env, shares);
        let share_key = DataKey::Shares(provider.clone(), token.clone());
        let queued_key = DataKey::QueuedShares(provider.clone(), token.clone());
        let owned = read_amount(&env, share_key);
        let queued = read_amount(&env, queued_key.clone());
        if owned - queued < shares {
            panic_with_error!(&env, ReserveError::InsufficientShares);
        }
        env.storage()
            .persistent()
            .set(&queued_key, &(queued + shares));
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextWithdrawalId)
            .unwrap_or(1);
        env.storage()
            .instance()
            .set(&DataKey::NextWithdrawalId, &(id + 1));
        env.storage().persistent().set(
            &DataKey::Withdrawal(id),
            &WithdrawalRequest {
                id,
                owner: provider,
                token,
                shares,
                executable_at: env.ledger().timestamp() + WITHDRAW_DELAY,
                completed: false,
                cancelled: false,
            },
        );
        id
    }

    pub fn cancel_withdraw(env: Env, provider: Address, request_id: u64) {
        provider.require_auth();
        let mut request = Self::get_withdrawal(env.clone(), request_id);
        if request.owner != provider {
            panic_with_error!(&env, ReserveError::NotOwner);
        }
        if request.completed || request.cancelled {
            panic_with_error!(&env, ReserveError::RequestClosed);
        }
        request.cancelled = true;
        env.storage()
            .persistent()
            .set(&DataKey::Withdrawal(request_id), &request);
        let key = DataKey::QueuedShares(provider, request.token);
        env.storage()
            .persistent()
            .set(&key, &(read_amount(&env, key.clone()) - request.shares));
    }

    pub fn execute_withdraw(
        env: Env,
        provider: Address,
        request_id: u64,
        min_amount: i128,
    ) -> i128 {
        provider.require_auth();
        let mut request = Self::get_withdrawal(env.clone(), request_id);
        if request.owner != provider {
            panic_with_error!(&env, ReserveError::NotOwner);
        }
        if request.completed || request.cancelled {
            panic_with_error!(&env, ReserveError::RequestClosed);
        }
        if env.ledger().timestamp() < request.executable_at {
            panic_with_error!(&env, ReserveError::RequestNotReady);
        }
        let mut state = pool(&env, &request.token);
        let nav = provider_nav(&state);
        let amount = request.shares * nav / state.total_shares;
        if amount < min_amount {
            panic_with_error!(&env, ReserveError::Slippage);
        }
        if amount <= 0 || amount > liquid_assets(&state) {
            panic_with_error!(&env, ReserveError::InsufficientLiquidity);
        }
        let share_key = DataKey::Shares(provider.clone(), request.token.clone());
        let queued_key = DataKey::QueuedShares(provider.clone(), request.token.clone());
        state.total_assets -= amount;
        state.total_shares -= request.shares;
        set_pool(&env, &state);
        env.storage().persistent().set(
            &share_key,
            &(read_amount(&env, share_key.clone()) - request.shares),
        );
        env.storage().persistent().set(
            &queued_key,
            &(read_amount(&env, queued_key.clone()) - request.shares),
        );
        request.completed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Withdrawal(request_id), &request);
        token::Client::new(&env, &request.token).transfer(
            &env.current_contract_address(),
            &provider,
            &amount,
        );
        amount
    }

    pub fn get_pool(env: Env, token: Address) -> PoolState {
        pool(&env, &token)
    }

    pub fn get_provider_shares(env: Env, provider: Address, token: Address) -> i128 {
        read_amount(&env, DataKey::Shares(provider, token))
    }

    pub fn get_available_liquidity(env: Env, token: Address) -> i128 {
        liquid_assets(&pool(&env, &token))
    }

    pub fn get_utilization_bps(env: Env, token: Address) -> u32 {
        let state = pool(&env, &token);
        let usable = provider_nav(&state);
        if usable <= 0 {
            return 10_000;
        }
        ((state.locked_liabilities * BPS / usable).min(BPS)) as u32
    }

    pub fn get_projected_utilization_bps(env: Env, token: Address, additional: i128) -> u32 {
        if additional < 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }
        let state = pool(&env, &token);
        let usable = provider_nav(&state);
        if usable <= 0 {
            return 10_000;
        }
        (((state.locked_liabilities + additional) * BPS / usable).min(BPS)) as u32
    }

    pub fn get_market_concentration_bps(
        env: Env,
        protected_asset: Address,
        payout_token: Address,
    ) -> u32 {
        let state = pool(&env, &payout_token);
        let usable = provider_nav(&state);
        if usable <= 0 {
            return 0;
        }
        let market = read_amount(&env, DataKey::MarketLocked(protected_asset, payout_token));
        ((market * BPS / usable).min(BPS)) as u32
    }

    pub fn get_projected_concentration_bps(
        env: Env,
        protected_asset: Address,
        payout_token: Address,
        additional: i128,
    ) -> u32 {
        if additional < 0 {
            panic_with_error!(&env, ReserveError::InvalidAmount);
        }
        let state = pool(&env, &payout_token);
        let usable = provider_nav(&state);
        if usable <= 0 {
            return 10_000;
        }
        let market = read_amount(&env, DataKey::MarketLocked(protected_asset, payout_token));
        (((market + additional) * BPS / usable).min(BPS)) as u32
    }

    pub fn get_provider_nav(env: Env, token: Address) -> i128 {
        provider_nav(&pool(&env, &token))
    }

    pub fn quote_shares(env: Env, token: Address, amount: i128) -> i128 {
        checked_positive(&env, amount);
        let state = pool(&env, &token);
        let nav = provider_nav(&state);
        if state.total_shares == 0 || nav <= 0 {
            amount
        } else {
            amount * state.total_shares / nav
        }
    }

    pub fn get_locked_for_position(env: Env, position_id: u64) -> i128 {
        read_amount(&env, DataKey::PositionLock(position_id))
    }

    pub fn get_claim(env: Env, position_id: u64) -> ClaimRecord {
        env.storage()
            .persistent()
            .get(&DataKey::Claim(position_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReserveError::ClaimNotFound))
    }

    pub fn get_withdrawal(env: Env, request_id: u64) -> WithdrawalRequest {
        env.storage()
            .persistent()
            .get(&DataKey::Withdrawal(request_id))
            .unwrap_or_else(|| panic_with_error!(&env, ReserveError::RequestClosed))
    }

    pub fn extend_ttl(env: Env) {
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
    }
}

#[cfg(test)]
mod test;
