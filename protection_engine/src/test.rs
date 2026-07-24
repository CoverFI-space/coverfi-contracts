extern crate std;

use super::{PositionStatus, ProtectionEngine, ProtectionEngineClient};
use oracle_adapter::{OracleAdapter, OracleAdapterClient};
use protected_balance_vault::{ProtectedBalanceVault, ProtectedBalanceVaultClient};
use reserve_vault::{ReserveVault, ReserveVaultClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, Symbol,
};
use zk_verifier::{ZkVerifier, ZkVerifierClient};

const ENTRY_PRICE: i128 = 10_000_000;
const SETTLEMENT_PRICE: i128 = 9_400_000;
const HIGH_PRICE: i128 = 10_100_000;
const AMOUNT: i128 = 10_000_000_000;
const NOTIONAL: i128 = 1_000_000_000;
const MAX_PAYOUT: i128 = 100_000_000;
const EXPECTED_PAYOUT: i128 = 60_000_000;
const DURATION: u64 = 7 * 86_400;
const DEFAULT_HIGH_VALUE_THRESHOLD: i128 = 10_000_000_000_000;
const MFA_CIRCUIT_ID: [u8; 32] = [
    0xd3, 0xd3, 0xbd, 0x01, 0x73, 0x8a, 0xc6, 0x4c, 0xc0, 0xad, 0x02, 0xa0, 0x74, 0xbb, 0x24,
    0xdb, 0x18, 0x8d, 0xf4, 0x25, 0xd1, 0xf5, 0x86, 0xaf, 0x4e, 0x7c, 0xce, 0x50, 0x50, 0x66,
    0x96, 0xeb,
];
const KYC_CIRCUIT_ID: [u8; 32] = [
    0x00, 0x84, 0xe5, 0xd6, 0x0f, 0xe2, 0x40, 0x45, 0x12, 0xe3, 0xec, 0xd6, 0x4f, 0xc3, 0x04,
    0xb8, 0x5e, 0x01, 0x73, 0x88, 0x53, 0x4c, 0x31, 0xee, 0x52, 0xe9, 0xb0, 0x0c, 0x8c, 0xf9,
    0x1a, 0x3e,
];

struct Fixture {
    env: Env,
    admin: Address,
    treasury: Address,
    user: Address,
    other: Address,
    publisher1: Address,
    publisher2: Address,
    engine_id: Address,
    protected_vault_id: Address,
    reserve_vault_id: Address,
    oracle_id: Address,
    zk_verifier_id: Address,
    protected_asset: Address,
    payout_asset: Address,
}

impl Fixture {
    fn engine(&self) -> ProtectionEngineClient<'_> {
        ProtectionEngineClient::new(&self.env, &self.engine_id)
    }

    fn protected_vault(&self) -> ProtectedBalanceVaultClient<'_> {
        ProtectedBalanceVaultClient::new(&self.env, &self.protected_vault_id)
    }

    fn reserve_vault(&self) -> ReserveVaultClient<'_> {
        ReserveVaultClient::new(&self.env, &self.reserve_vault_id)
    }

    fn oracle(&self) -> OracleAdapterClient<'_> {
        OracleAdapterClient::new(&self.env, &self.oracle_id)
    }

    fn zk_verifier(&self) -> ZkVerifierClient<'_> {
        ZkVerifierClient::new(&self.env, &self.zk_verifier_id)
    }

    fn protected_token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.protected_asset)
    }

    fn payout_token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.payout_asset)
    }

    fn submit_price(&self, timestamp: u64, price: i128) {
        assert!(!self.oracle().submit_fallback(
            &self.publisher1,
            &self.protected_asset,
            &price,
            &timestamp,
        ));
        assert!(self.oracle().submit_fallback(
            &self.publisher2,
            &self.protected_asset,
            &price,
            &timestamp,
        ));
    }

    fn create_position(&self) -> u64 {
        self.engine().create_position(
            &self.user,
            &self.protected_asset,
            &self.payout_asset,
            &AMOUNT,
            &DURATION,
            &None,
        )
    }

    fn move_to_expiry_with_price(&self, price: i128) {
        self.env.ledger().set_timestamp(1_000 + DURATION);
        self.submit_price(1_000 + DURATION, price);
    }

    fn anchor_compliance_proofs(&self) {
        let mfa_circuit_id = BytesN::from_array(&self.env, &MFA_CIRCUIT_ID);
        let kyc_circuit_id = BytesN::from_array(&self.env, &KYC_CIRCUIT_ID);
        let verifier_hash = bytes(&self.env, 9);
        self.zk_verifier().admin_record_proof(
            &self.admin,
            &self.user,
            &mfa_circuit_id,
            &bytes(&self.env, 10),
            &bytes(&self.env, 11),
            &bytes(&self.env, 12),
            &verifier_hash,
        );
        self.zk_verifier().admin_record_proof(
            &self.admin,
            &self.user,
            &kyc_circuit_id,
            &bytes(&self.env, 20),
            &bytes(&self.env, 21),
            &bytes(&self.env, 22),
            &verifier_hash,
        );
    }
}

fn setup(reserve_amount: i128) -> Fixture {
    setup_with_threshold(reserve_amount, DEFAULT_HIGH_VALUE_THRESHOLD)
}

fn bytes(env: &Env, value: u8) -> BytesN<32> {
    BytesN::from_array(env, &[value; 32])
}

fn setup_with_threshold(reserve_amount: i128, high_value_threshold: i128) -> Fixture {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let user = Address::generate(&env);
    let other = Address::generate(&env);
    let publisher1 = Address::generate(&env);
    let publisher2 = Address::generate(&env);
    let publisher3 = Address::generate(&env);
    let source = Address::generate(&env);

    let protected_asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let payout_asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &protected_asset).mint(&user, &100_000_000_000);
    token::StellarAssetClient::new(&env, &payout_asset).mint(&admin, &reserve_amount);
    token::StellarAssetClient::new(&env, &payout_asset).mint(&user, &1_000_000_000);

    let protected_vault_id = env.register(ProtectedBalanceVault, ());
    let reserve_vault_id = env.register(ReserveVault, ());
    let oracle_id = env.register(OracleAdapter, ());
    let zk_verifier_id = env.register(ZkVerifier, ());
    let engine_id = env.register(ProtectionEngine, ());

    let protected_vault = ProtectedBalanceVaultClient::new(&env, &protected_vault_id);
    let reserve_vault = ReserveVaultClient::new(&env, &reserve_vault_id);
    let oracle = OracleAdapterClient::new(&env, &oracle_id);
    let zk_verifier = ZkVerifierClient::new(&env, &zk_verifier_id);
    let engine = ProtectionEngineClient::new(&env, &engine_id);

    protected_vault.initialize(&admin, &engine_id);
    reserve_vault.initialize(&admin, &treasury);
    reserve_vault.configure_protocol(&admin, &engine_id, &oracle_id);
    oracle.initialize(
        &admin,
        &source,
        &Symbol::new(&env, "XLM"),
        &8,
        &3_600,
        &2_000,
    );
    oracle.configure_publishers(&admin, &publisher1, &publisher2, &publisher3, &true);
    zk_verifier.initialize(&admin);
    let verifier_hash = bytes(&env, 9);
    let kind_hash = bytes(&env, 8);
    zk_verifier.register_verifier(
        &admin,
        &BytesN::from_array(&env, &MFA_CIRCUIT_ID),
        &verifier_hash,
        &kind_hash,
        &true,
    );
    zk_verifier.register_verifier(
        &admin,
        &BytesN::from_array(&env, &KYC_CIRCUIT_ID),
        &verifier_hash,
        &kind_hash,
        &true,
    );
    engine.initialize(
        &admin,
        &protected_vault_id,
        &reserve_vault_id,
        &oracle_id,
        &zk_verifier_id,
        &protected_asset,
        &payout_asset,
        &1_000,
        &3_600,
        &high_value_threshold,
    );
    if reserve_amount > 0 {
        reserve_vault.deposit_reserve(&admin, &payout_asset, &reserve_amount, &reserve_amount);
    }

    let fixture = Fixture {
        env,
        admin,
        treasury,
        user,
        other,
        publisher1,
        publisher2,
        engine_id,
        protected_vault_id,
        reserve_vault_id,
        oracle_id,
        zk_verifier_id,
        protected_asset,
        payout_asset,
    };
    fixture.submit_price(1_000, ENTRY_PRICE);
    fixture
}

#[test]
fn quote_exposes_all_price_components() {
    let fixture = setup(100_000_000_000);
    let quote = fixture.engine().quote_position(
        &fixture.protected_asset,
        &fixture.payout_asset,
        &AMOUNT,
        &DURATION,
    );
    assert_eq!(quote.entry_price, ENTRY_PRICE);
    assert_eq!(quote.notional, NOTIONAL);
    assert_eq!(quote.maximum_payout, MAX_PAYOUT);
    assert_eq!(quote.base_premium, 10_000_000);
    assert_eq!(quote.volatility_surcharge, 0);
    assert_eq!(quote.utilization_surcharge, 0);
    assert_eq!(quote.concentration_surcharge, 0);
    assert_eq!(quote.safety_margin, 2_000_000);
    assert_eq!(quote.risk_premium, 12_000_000);
    assert_eq!(quote.protocol_commission, 1_440_000);
    assert_eq!(quote.automation_fee, 500_000);
    assert_eq!(quote.total_due, 12_500_000);
}

#[test]
fn creation_segregates_principal_routes_fees_and_locks_full_cap() {
    let fixture = setup(100_000_000_000);
    let id = fixture.create_position();
    let value = fixture.engine().get_position(&id);
    let pool = fixture.reserve_vault().get_pool(&fixture.payout_asset);

    assert_eq!(value.status, PositionStatus::Active);
    assert_eq!(value.notional, NOTIONAL);
    assert_eq!(value.maximum_payout, MAX_PAYOUT);
    assert_eq!(fixture.protected_vault().get_position_balance(&id), AMOUNT);
    assert_eq!(
        fixture.reserve_vault().get_locked_for_position(&id),
        MAX_PAYOUT
    );
    assert_eq!(pool.unearned_premiums, 9_360_000);
    assert_eq!(pool.safety_balance, 840_000);
    assert_eq!(pool.automation_balance, 860_000);
    assert_eq!(fixture.payout_token().balance(&fixture.treasury), 1_440_000);
    assert_eq!(
        fixture.protected_token().balance(&fixture.user),
        90_000_000_000
    );
    assert_eq!(fixture.payout_token().balance(&fixture.user), 987_500_000);
}

#[test]
fn expiry_settlement_pays_full_stable_value_claim_without_epoch() {
    let fixture = setup(100_000_000_000);
    let id = fixture.create_position();
    fixture.move_to_expiry_with_price(SETTLEMENT_PRICE);

    let result = fixture.engine().settle_position(&fixture.other, &id);
    assert_eq!(result.status, PositionStatus::Claimable);
    assert_eq!(result.payout, EXPECTED_PAYOUT);
    assert!(result.keeper_rewarded);
    assert_eq!(fixture.reserve_vault().get_locked_for_position(&id), 0);
    assert_eq!(
        fixture
            .reserve_vault()
            .get_pool(&fixture.payout_asset)
            .reserved_claims,
        EXPECTED_PAYOUT
    );

    assert_eq!(
        fixture.engine().claim_payout(&fixture.user, &id),
        EXPECTED_PAYOUT
    );
    assert_eq!(
        fixture.payout_token().balance(&fixture.user),
        987_500_000 + EXPECTED_PAYOUT
    );
    assert_eq!(
        fixture
            .reserve_vault()
            .get_pool(&fixture.payout_asset)
            .reserved_claims,
        0
    );
}

#[test]
#[should_panic]
fn high_value_claim_requires_mfa_and_kyc_proofs() {
    let fixture = setup_with_threshold(100_000_000_000, NOTIONAL);
    let id = fixture.create_position();
    fixture.move_to_expiry_with_price(SETTLEMENT_PRICE);
    fixture.engine().settle_position(&fixture.other, &id);
    fixture.engine().claim_payout(&fixture.user, &id);
}

#[test]
fn high_value_claim_and_principal_withdrawal_accept_anchored_mfa_and_kyc_proofs() {
    let fixture = setup_with_threshold(100_000_000_000, NOTIONAL);
    let id = fixture.create_position();
    fixture.move_to_expiry_with_price(SETTLEMENT_PRICE);
    fixture.engine().settle_position(&fixture.other, &id);
    fixture.anchor_compliance_proofs();

    assert_eq!(
        fixture.engine().claim_payout(&fixture.user, &id),
        EXPECTED_PAYOUT
    );
    assert_eq!(
        fixture.engine().withdraw_principal(&fixture.user, &id),
        AMOUNT
    );
}

#[test]
fn missing_expiry_observation_waits_without_releasing_collateral() {
    let fixture = setup(100_000_000_000);
    let id = fixture.create_position();
    fixture.env.ledger().set_timestamp(1_000 + DURATION);

    let waiting = fixture.engine().settle_position(&fixture.other, &id);
    assert_eq!(waiting.status, PositionStatus::AwaitingOracle);
    assert_eq!(
        fixture.reserve_vault().get_locked_for_position(&id),
        MAX_PAYOUT
    );

    fixture.submit_price(1_000 + DURATION, SETTLEMENT_PRICE);
    let settled = fixture.engine().settle_position(&fixture.other, &id);
    assert_eq!(settled.status, PositionStatus::Claimable);
}

#[test]
fn no_loss_settlement_matures_premium_and_releases_collateral() {
    let fixture = setup(100_000_000_000);
    let id = fixture.create_position();
    fixture.move_to_expiry_with_price(HIGH_PRICE);
    let settled = fixture.engine().settle_position(&fixture.other, &id);
    assert_eq!(settled.status, PositionStatus::SettledNoPayout);
    assert_eq!(settled.payout, 0);
    assert_eq!(fixture.reserve_vault().get_locked_for_position(&id), 0);
    assert_eq!(
        fixture
            .reserve_vault()
            .get_pool(&fixture.payout_asset)
            .unearned_premiums,
        0
    );

    assert_eq!(
        fixture.engine().withdraw_principal(&fixture.user, &id),
        AMOUNT
    );
    assert_eq!(
        fixture.engine().get_position(&id).status,
        PositionStatus::PrincipalWithdrawn
    );
    assert_eq!(
        fixture.protected_token().balance(&fixture.user),
        100_000_000_000
    );
}

#[test]
fn pause_stops_sales_but_not_settlement_claims_or_principal() {
    let fixture = setup(100_000_000_000);
    let id = fixture.create_position();
    fixture.move_to_expiry_with_price(SETTLEMENT_PRICE);
    fixture.engine().pause(&fixture.admin);
    fixture.engine().settle_position(&fixture.other, &id);
    fixture.engine().withdraw_principal(&fixture.user, &id);
    assert_eq!(
        fixture.engine().claim_payout(&fixture.user, &id),
        EXPECTED_PAYOUT
    );
}

#[test]
#[should_panic]
fn cannot_sell_above_reserve_risk_limits() {
    let fixture = setup(MAX_PAYOUT);
    fixture.create_position();
}

#[test]
#[should_panic]
fn cannot_claim_twice() {
    let fixture = setup(100_000_000_000);
    let id = fixture.create_position();
    fixture.move_to_expiry_with_price(SETTLEMENT_PRICE);
    fixture.engine().settle_position(&fixture.other, &id);
    fixture.engine().claim_payout(&fixture.user, &id);
    fixture.engine().claim_payout(&fixture.user, &id);
}

#[test]
#[should_panic]
fn cannot_withdraw_principal_before_settlement() {
    let fixture = setup(100_000_000_000);
    let id = fixture.create_position();
    fixture.engine().withdraw_principal(&fixture.user, &id);
}

#[test]
#[should_panic]
fn unsupported_market_is_rejected() {
    let fixture = setup(100_000_000_000);
    fixture.engine().create_position(
        &fixture.user,
        &fixture.payout_asset,
        &fixture.protected_asset,
        &AMOUNT,
        &DURATION,
        &None,
    );
}
