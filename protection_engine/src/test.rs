extern crate std;

use super::{PositionStatus, ProtectionEngine, ProtectionEngineClient};
use oracle_adapter::{OracleAdapter, OracleAdapterClient};
use premium_vault::{PremiumVault, PremiumVaultClient};
use protected_balance_vault::{ProtectedBalanceVault, ProtectedBalanceVaultClient};
use reserve_vault::{ReserveVault, ReserveVaultClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

const PEG: i128 = 100_000_000;
const TRIGGER_098: i128 = 98_000_000;
const PRICE_099: i128 = 99_000_000;
const PRICE_094: i128 = 94_000_000;
const AMOUNT: i128 = 1_000;
const FEE: i128 = 10;
const MAX_PAYOUT: i128 = 100;
const DURATION: u64 = 604_800;

struct Fixture {
    env: Env,
    admin: Address,
    user: Address,
    other: Address,
    engine_id: Address,
    protected_vault_id: Address,
    premium_vault_id: Address,
    reserve_vault_id: Address,
    oracle_id: Address,
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

    fn premium_vault(&self) -> PremiumVaultClient<'_> {
        PremiumVaultClient::new(&self.env, &self.premium_vault_id)
    }

    fn reserve_vault(&self) -> ReserveVaultClient<'_> {
        ReserveVaultClient::new(&self.env, &self.reserve_vault_id)
    }

    fn oracle(&self) -> OracleAdapterClient<'_> {
        OracleAdapterClient::new(&self.env, &self.oracle_id)
    }

    fn protected_token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.protected_asset)
    }

    fn payout_token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.payout_asset)
    }

    fn create_position(&self) -> u64 {
        self.engine().create_position(
            &self.user,
            &self.protected_asset,
            &self.payout_asset,
            &AMOUNT,
            &DURATION,
            &TRIGGER_098,
        )
    }
}

fn setup(reserve_amount: i128) -> Fixture {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let other = Address::generate(&env);

    let protected_asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let payout_asset = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &protected_asset).mint(&user, &10_000);
    token::StellarAssetClient::new(&env, &payout_asset).mint(&admin, &10_000);

    let protected_vault_id = env.register(ProtectedBalanceVault, ());
    let premium_vault_id = env.register(PremiumVault, ());
    let reserve_vault_id = env.register(ReserveVault, ());
    let oracle_id = env.register(OracleAdapter, ());
    let engine_id = env.register(ProtectionEngine, ());

    let protected_vault = ProtectedBalanceVaultClient::new(&env, &protected_vault_id);
    let premium_vault = PremiumVaultClient::new(&env, &premium_vault_id);
    let reserve_vault = ReserveVaultClient::new(&env, &reserve_vault_id);
    let oracle = OracleAdapterClient::new(&env, &oracle_id);
    let engine = ProtectionEngineClient::new(&env, &engine_id);

    protected_vault.initialize(&admin, &engine_id);
    reserve_vault.initialize(&admin, &engine_id);
    premium_vault.initialize(&admin, &engine_id, &reserve_vault_id);
    oracle.initialize(&admin);
    engine.initialize(
        &admin,
        &protected_vault_id,
        &premium_vault_id,
        &reserve_vault_id,
        &oracle_id,
        &100,
        &1_000,
    );
    oracle.set_price(&admin, &protected_asset, &PEG);
    if reserve_amount > 0 {
        reserve_vault.deposit_reserve(&admin, &payout_asset, &reserve_amount);
    }

    Fixture {
        env,
        admin,
        user,
        other,
        engine_id,
        protected_vault_id,
        premium_vault_id,
        reserve_vault_id,
        oracle_id,
        protected_asset,
        payout_asset,
    }
}

#[test]
fn position_creation_moves_funds_and_locks_reserve() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    let position = fixture.engine().get_position(&position_id);

    assert_eq!(position.status, PositionStatus::Active);
    assert_eq!(position.protected_amount, AMOUNT);
    assert_eq!(position.fee_paid, FEE);
    assert_eq!(
        fixture.protected_vault().get_position_balance(&position_id),
        AMOUNT
    );
    assert_eq!(
        fixture
            .protected_vault()
            .get_total_token_balance(&fixture.protected_asset),
        AMOUNT
    );
    assert_eq!(
        fixture
            .premium_vault()
            .get_total_premiums(&fixture.protected_asset),
        FEE
    );
    assert_eq!(
        fixture
            .reserve_vault()
            .get_total_reserve(&fixture.protected_asset),
        FEE
    );
    assert_eq!(
        fixture
            .reserve_vault()
            .get_locked_for_position(&position_id),
        MAX_PAYOUT
    );
    assert_eq!(
        fixture
            .reserve_vault()
            .get_locked_reserve(&fixture.payout_asset),
        MAX_PAYOUT
    );
    assert_eq!(
        fixture.protected_token().balance(&fixture.user),
        10_000 - AMOUNT - FEE
    );

    let user_positions = fixture.engine().get_user_positions(&fixture.user);
    assert_eq!(user_positions.len(), 1);
    assert_eq!(user_positions.get(0).unwrap(), position_id);
}

#[test]
fn expiry_releases_locked_capacity_and_principal_once() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();

    fixture.env.ledger().set_timestamp(1_000 + DURATION + 1);
    fixture.engine().expire_position(&position_id);
    assert_eq!(
        fixture.engine().get_position(&position_id).status,
        PositionStatus::Expired
    );
    assert_eq!(
        fixture
            .reserve_vault()
            .get_locked_for_position(&position_id),
        0
    );
    assert_eq!(
        fixture
            .reserve_vault()
            .get_locked_reserve(&fixture.payout_asset),
        0
    );

    fixture
        .engine()
        .withdraw_principal(&fixture.user, &position_id);
    let position = fixture.engine().get_position(&position_id);
    assert!(position.principal_withdrawn);
    assert_eq!(
        fixture.protected_vault().get_position_balance(&position_id),
        0
    );
    assert_eq!(
        fixture.protected_token().balance(&fixture.user),
        10_000 - FEE
    );
}

#[test]
#[should_panic]
fn user_cannot_withdraw_principal_twice() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    fixture.env.ledger().set_timestamp(1_000 + DURATION + 1);
    fixture.engine().expire_position(&position_id);
    fixture
        .engine()
        .withdraw_principal(&fixture.user, &position_id);
    fixture
        .engine()
        .withdraw_principal(&fixture.user, &position_id);
}

#[test]
fn trigger_claim_and_principal_withdrawal_flow() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    fixture
        .oracle()
        .set_price(&fixture.admin, &fixture.protected_asset, &PRICE_094);

    fixture.engine().check_and_trigger(&position_id);
    let triggered = fixture.engine().get_position(&position_id);
    assert_eq!(triggered.status, PositionStatus::Triggered);
    assert_eq!(triggered.claimable_payout, 60);

    fixture.engine().claim_payout(&fixture.user, &position_id);
    let claimed = fixture.engine().get_position(&position_id);
    assert_eq!(claimed.status, PositionStatus::Claimed);
    assert_eq!(fixture.payout_token().balance(&fixture.user), 60);
    assert_eq!(
        fixture
            .reserve_vault()
            .get_total_reserve(&fixture.payout_asset),
        940
    );
    assert_eq!(
        fixture
            .reserve_vault()
            .get_locked_for_position(&position_id),
        0
    );

    fixture
        .engine()
        .withdraw_principal(&fixture.user, &position_id);
    assert_eq!(
        fixture.protected_token().balance(&fixture.user),
        10_000 - FEE
    );
}

#[test]
#[should_panic]
fn user_cannot_claim_twice() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    fixture
        .oracle()
        .set_price(&fixture.admin, &fixture.protected_asset, &PRICE_094);
    fixture.engine().check_and_trigger(&position_id);
    fixture.engine().claim_payout(&fixture.user, &position_id);
    fixture.engine().claim_payout(&fixture.user, &position_id);
}

#[test]
#[should_panic]
fn price_above_trigger_does_not_trigger() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    fixture
        .oracle()
        .set_price(&fixture.admin, &fixture.protected_asset, &PRICE_099);
    fixture.engine().check_and_trigger(&position_id);
}

#[test]
fn no_trigger_expires_normally() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    fixture
        .oracle()
        .set_price(&fixture.admin, &fixture.protected_asset, &PRICE_099);
    fixture.env.ledger().set_timestamp(1_000 + DURATION + 1);
    fixture.engine().expire_position(&position_id);
    assert_eq!(
        fixture.engine().get_position(&position_id).status,
        PositionStatus::Expired
    );
}

#[test]
#[should_panic]
fn non_owner_cannot_claim_another_users_payout() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    fixture
        .oracle()
        .set_price(&fixture.admin, &fixture.protected_asset, &PRICE_094);
    fixture.engine().check_and_trigger(&position_id);
    fixture.engine().claim_payout(&fixture.other, &position_id);
}

#[test]
#[should_panic]
fn non_owner_cannot_withdraw_another_users_principal() {
    let fixture = setup(1_000);
    let position_id = fixture.create_position();
    fixture.env.ledger().set_timestamp(1_000 + DURATION + 1);
    fixture.engine().expire_position(&position_id);
    fixture
        .engine()
        .withdraw_principal(&fixture.other, &position_id);
}

#[test]
#[should_panic]
fn non_admin_cannot_set_oracle_price() {
    let fixture = setup(1_000);
    fixture
        .oracle()
        .set_price(&fixture.other, &fixture.protected_asset, &PRICE_094);
}

#[test]
#[should_panic]
fn insufficient_reserve_blocks_position_creation() {
    let fixture = setup(50);
    fixture.create_position();
}

#[test]
#[should_panic]
fn invalid_amount_is_rejected() {
    let fixture = setup(1_000);
    fixture.engine().create_position(
        &fixture.user,
        &fixture.protected_asset,
        &fixture.payout_asset,
        &0,
        &DURATION,
        &TRIGGER_098,
    );
}

#[test]
#[should_panic]
fn invalid_trigger_is_rejected() {
    let fixture = setup(1_000);
    fixture.engine().create_position(
        &fixture.user,
        &fixture.protected_asset,
        &fixture.payout_asset,
        &AMOUNT,
        &DURATION,
        &(PEG + 1),
    );
}
