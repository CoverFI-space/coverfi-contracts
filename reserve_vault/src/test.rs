extern crate std;

use super::{ReserveVault, ReserveVaultClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

struct Fixture {
    env: Env,
    admin: Address,
    engine: Address,
    user: Address,
    token: Address,
    vault_id: Address,
}

impl Fixture {
    fn vault(&self) -> ReserveVaultClient<'_> {
        ReserveVaultClient::new(&self.env, &self.vault_id)
    }

    fn token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.token)
    }
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let engine = Address::generate(&env);
    let user = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&admin, &10_000);

    let vault_id = env.register(ReserveVault, ());
    let vault = ReserveVaultClient::new(&env, &vault_id);
    vault.initialize(&admin, &engine);
    vault.deposit_reserve(&admin, &token, &1_000);

    Fixture {
        env,
        admin,
        engine,
        user,
        token,
        vault_id,
    }
}

#[test]
fn restricted_payout_reserves_before_withdrawal() {
    let fixture = setup();
    let vault = fixture.vault();

    assert_eq!(vault.get_current_epoch(), 1);
    assert_eq!(vault.get_floor_bps(), 2_000);
    assert_eq!(vault.get_drain_bps(), 5_000);
    vault.lock_payout_capacity(&fixture.engine, &fixture.token, &1, &100);
    vault.pay_claim(&fixture.engine, &fixture.token, &fixture.user, &1, &60);

    assert_eq!(fixture.token().balance(&fixture.user), 0);
    assert_eq!(vault.get_epoch_total(&1, &fixture.token), 60);
    assert_eq!(vault.get_locked_for_position(&1), 0);
    assert_eq!(vault.get_locked_reserve(&fixture.token), 0);
    assert_eq!(vault.close_epoch(&fixture.admin, &fixture.token), 60);
    assert_eq!(vault.get_reserved_reserve(&fixture.token), 60);
    assert_eq!(vault.quote_withdraw(&1), 60);
    assert_eq!(vault.withdraw_reserved(&fixture.user, &1), 60);
    assert_eq!(fixture.token().balance(&fixture.user), 60);
    assert_eq!(vault.get_total_reserve(&fixture.token), 940);
    assert_eq!(vault.get_reserved_reserve(&fixture.token), 0);
}

#[test]
fn floor_and_drain_policy_throttle_epoch_allocation() {
    let fixture = setup();
    let vault = fixture.vault();

    vault.set_restricted_payout_policy(&fixture.admin, &2_000, &5_000);
    vault.lock_payout_capacity(&fixture.engine, &fixture.token, &1, &900);
    vault.pay_claim(&fixture.engine, &fixture.token, &fixture.user, &1, &900);

    assert_eq!(vault.close_epoch(&fixture.admin, &fixture.token), 400);
    assert_eq!(vault.quote_withdraw(&1), 400);
    assert_eq!(vault.get_available_reserve(&fixture.token), 600);
    assert_eq!(vault.get_floor_bps(), 2_000);
    assert_eq!(vault.get_drain_bps(), 5_000);
}

#[test]
fn partially_funded_epoch_can_receive_later_allocations() {
    let fixture = setup();
    let vault = fixture.vault();

    vault.set_restricted_payout_policy(&fixture.admin, &2_000, &5_000);
    vault.lock_payout_capacity(&fixture.engine, &fixture.token, &1, &900);
    vault.pay_claim(&fixture.engine, &fixture.token, &fixture.user, &1, &900);

    assert_eq!(vault.close_epoch(&fixture.admin, &fixture.token), 400);
    assert_eq!(vault.get_current_epoch(), 2);
    assert_eq!(vault.quote_withdraw(&1), 400);

    vault.deposit_reserve(&fixture.admin, &fixture.token, &1_000);
    assert_eq!(
        vault.allocate_epoch(&fixture.admin, &1, &fixture.token),
        500
    );
    assert_eq!(vault.quote_withdraw(&1), 900);
}
