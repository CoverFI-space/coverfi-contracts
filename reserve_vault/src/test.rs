extern crate std;

use super::{ReserveVault, ReserveVaultClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Symbol,
};

struct Fixture {
    env: Env,
    admin: Address,
    engine: Address,
    oracle: Address,
    treasury: Address,
    provider: Address,
    user: Address,
    partner: Address,
    token: Address,
    vault_id: Address,
}

impl Fixture {
    fn vault(&self) -> ReserveVaultClient<'_> {
        ReserveVaultClient::new(&self.env, &self.vault_id)
    }

    fn balance(&self, owner: &Address) -> i128 {
        token::Client::new(&self.env, &self.token).balance(owner)
    }
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(1_000);
    let admin = Address::generate(&env);
    let engine = Address::generate(&env);
    let oracle = Address::generate(&env);
    let treasury = Address::generate(&env);
    let provider = Address::generate(&env);
    let user = Address::generate(&env);
    let partner = Address::generate(&env);
    let protected_asset = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let asset = token::StellarAssetClient::new(&env, &token);
    asset.mint(&provider, &20_000_000_000);
    asset.mint(&user, &1_000_000_000);
    let vault_id = env.register(ReserveVault, ());
    let vault = ReserveVaultClient::new(&env, &vault_id);
    vault.initialize(&admin, &treasury);
    vault.configure_protocol(&admin, &engine, &oracle);
    vault.set_partner(&admin, &partner, &true);
    vault.deposit_reserve(&provider, &token, &10_000_000_000, &10_000_000_000);
    vault.lock_payout_capacity(&engine, &protected_asset, &token, &1, &1_000_000_000);
    Fixture {
        env,
        admin,
        engine,
        oracle,
        treasury,
        provider,
        user,
        partner,
        token,
        vault_id,
    }
}

#[test]
fn spoofed_engine_cannot_lock_or_settle_reserve() {
    let fixture = setup();
    let before = fixture.vault().get_pool(&fixture.token);
    fixture.env.set_auths(&[]);
    // Supplying the right address is not authorization from that address.
    assert!(fixture
        .vault()
        .try_settle_position(&fixture.engine, &fixture.user, &1, &1)
        .is_err());
    assert!(fixture
        .vault()
        .try_lock_payout_capacity(&fixture.engine, &fixture.token, &fixture.token, &2, &1,)
        .is_err());
    assert_eq!(fixture.vault().get_pool(&fixture.token), before);
}

#[test]
fn spoofed_engine_cannot_consume_user_premium_or_claim() {
    let fixture = setup();
    fixture
        .vault()
        .settle_position(&fixture.engine, &fixture.user, &1, &100_000_000);
    let before = fixture.balance(&fixture.user);
    fixture.env.set_auths(&[]);
    assert!(fixture
        .vault()
        .try_collect_premium_from(
            &fixture.engine,
            &fixture.user,
            &fixture.token,
            &100_000_000,
            &500_000,
            &2,
            &None,
        )
        .is_err());
    assert!(fixture
        .vault()
        .try_claim_payout(&fixture.engine, &fixture.user, &1)
        .is_err());
    assert_eq!(fixture.balance(&fixture.user), before);
    assert_eq!(
        fixture.vault().get_pool(&fixture.token).reserved_claims,
        100_000_000
    );
}

#[test]
fn spoofed_reward_caller_cannot_drain_automation() {
    let fixture = setup();
    fixture
        .vault()
        .fund_automation(&fixture.provider, &fixture.token, &1_000_000);
    let before = fixture.vault().get_pool(&fixture.token);
    fixture.env.set_auths(&[]);
    for caller in [&fixture.engine, &fixture.oracle] {
        assert!(fixture
            .vault()
            .try_pay_keeper(
                caller,
                &fixture.token,
                &fixture.user,
                &Symbol::new(&fixture.env, "settlement"),
                &7,
                &100_000,
            )
            .is_err());
    }
    assert_eq!(fixture.vault().get_pool(&fixture.token), before);
}

#[test]
fn premium_split_is_conserved_and_visible() {
    let fixture = setup();
    let vault = fixture.vault();
    vault.collect_premium_from(
        &fixture.engine,
        &fixture.user,
        &fixture.token,
        &100_000_000,
        &500_000,
        &1,
        &Some(fixture.partner.clone()),
    );
    let state = vault.get_pool(&fixture.token);
    assert_eq!(state.unearned_premiums, 78_000_000);
    assert_eq!(state.safety_balance, 7_000_000);
    assert_eq!(state.automation_balance, 3_500_000);
    assert_eq!(state.cumulative_protocol_revenue, 12_000_000);
    assert_eq!(fixture.balance(&fixture.partner), 2_400_000);
    assert_eq!(fixture.balance(&fixture.treasury), 9_600_000);
}

#[test]
fn admin_can_route_treasury_and_bound_protocol_fee() {
    let fixture = setup();
    let vault = fixture.vault();
    let new_treasury = Address::generate(&fixture.env);
    vault.set_treasury(&fixture.admin, &new_treasury);
    vault.set_protocol_fee_bps(&fixture.admin, &1_500);

    vault.collect_premium_from(
        &fixture.engine,
        &fixture.user,
        &fixture.token,
        &100_000_000,
        &500_000,
        &2,
        &None,
    );

    let state = vault.get_pool(&fixture.token);
    assert_eq!(vault.get_treasury(), new_treasury);
    assert_eq!(vault.get_protocol_fee_bps(), 1_500);
    assert_eq!(state.unearned_premiums, 78_000_000);
    assert_eq!(state.safety_balance, 7_000_000);
    assert_eq!(state.automation_balance, 500_000);
    assert_eq!(state.cumulative_protocol_revenue, 15_000_000);
    assert_eq!(fixture.balance(&new_treasury), 15_000_000);
}

#[test]
fn claim_is_fully_reserved_and_paid_without_epoch() {
    let fixture = setup();
    let vault = fixture.vault();
    vault.collect_premium_from(
        &fixture.engine,
        &fixture.user,
        &fixture.token,
        &100_000_000,
        &500_000,
        &1,
        &None,
    );
    vault.settle_position(&fixture.engine, &fixture.user, &1, &600_000_000);
    assert_eq!(vault.get_locked_for_position(&1), 0);
    assert_eq!(vault.get_pool(&fixture.token).reserved_claims, 600_000_000);
    assert_eq!(
        vault.claim_payout(&fixture.engine, &fixture.user, &1),
        600_000_000
    );
    assert_eq!(
        fixture.balance(&fixture.user),
        1_000_000_000 - 100_500_000 + 600_000_000
    );
    assert_eq!(vault.get_pool(&fixture.token).reserved_claims, 0);
}

#[test]
fn queued_withdrawal_cannot_use_locked_capital() {
    let fixture = setup();
    let vault = fixture.vault();
    let request = vault.request_withdraw(&fixture.provider, &fixture.token, &9_500_000_000);
    fixture.env.ledger().set_timestamp(1_000 + 7 * 86_400);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        vault.execute_withdraw(&fixture.provider, &request, &0);
    }));
    assert!(result.is_err());
}

#[test]
fn keeper_reward_is_single_use() {
    let fixture = setup();
    let vault = fixture.vault();
    vault.fund_automation(&fixture.provider, &fixture.token, &10_000_000);
    vault.pay_keeper(
        &fixture.oracle,
        &fixture.token,
        &fixture.user,
        &Symbol::new(&fixture.env, "oracle"),
        &1,
        &1_000_000,
    );
    assert_eq!(fixture.balance(&fixture.user), 1_001_000_000);
}
