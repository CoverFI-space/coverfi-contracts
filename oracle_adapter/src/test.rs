use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, Symbol};

struct Fixture {
    env: Env,
    admin: Address,
    asset: Address,
    publisher1: Address,
    publisher2: Address,
    publisher3: Address,
    client: OracleAdapterClient<'static>,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let admin = Address::generate(&env);
    let source = Address::generate(&env);
    let asset = Address::generate(&env);
    let publisher1 = Address::generate(&env);
    let publisher2 = Address::generate(&env);
    let publisher3 = Address::generate(&env);
    let contract = env.register(OracleAdapter, ());
    let client = OracleAdapterClient::new(&env, &contract);
    client.initialize(
        &admin,
        &Some(source.clone()),
        &Symbol::new(&env, "USDC"),
        &8,
        &600,
        &2_000,
        &asset,
        &Address::generate(&env),
    );
    client.configure_publishers(&admin, &publisher1, &publisher2, &publisher3, &true);
    Fixture {
        env,
        admin,
        asset,
        publisher1,
        publisher2,
        publisher3,
        client,
    }
}

#[test]
fn fallback_accepts_any_matching_pair() {
    let fixture = setup();
    assert!(!fixture.client.submit_fallback(
        &fixture.publisher1,
        &fixture.asset,
        &19_000_000,
        &1_000,
    ));
    assert!(!fixture.client.submit_fallback(
        &fixture.publisher2,
        &fixture.asset,
        &18_500_000,
        &1_000,
    ));
    assert!(fixture.client.submit_fallback(
        &fixture.publisher3,
        &fixture.asset,
        &18_500_000,
        &1_000,
    ));
    assert_eq!(
        fixture.client.get_latest(&fixture.asset),
        Some(PriceObservation {
            price: 18_500_000,
            timestamp: 1_000,
        })
    );
}

#[test]
#[should_panic]
fn publishers_must_be_distinct() {
    let fixture = setup();
    fixture.client.configure_publishers(
        &fixture.admin,
        &fixture.publisher1,
        &fixture.publisher1,
        &fixture.publisher3,
        &true,
    );
}

#[test]
#[should_panic]
fn reconfiguration_revokes_old_publishers() {
    let fixture = setup();
    let replacement1 = Address::generate(&fixture.env);
    let replacement2 = Address::generate(&fixture.env);
    let replacement3 = Address::generate(&fixture.env);
    fixture.client.configure_publishers(
        &fixture.admin,
        &replacement1,
        &replacement2,
        &replacement3,
        &true,
    );
    fixture
        .client
        .submit_fallback(&fixture.publisher1, &fixture.asset, &19_000_000, &1_000);
}

#[test]
fn single_publisher_cannot_create_a_price() {
    let fixture = setup();
    assert!(!fixture.client.submit_fallback(
        &fixture.publisher1,
        &fixture.asset,
        &100_000_000,
        &1_000
    ));
    assert_eq!(fixture.client.get_fresh_price(&fixture.asset), None);
}

#[test]
fn publisher_only_mode_has_no_external_refresh_path() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let asset = Address::generate(&env);
    let payout = Address::generate(&env);
    let id = env.register(OracleAdapter, ());
    let client = OracleAdapterClient::new(&env, &id);
    client.initialize(
        &admin,
        &None,
        &Symbol::new(&env, "USDC"),
        &8,
        &600,
        &10_000,
        &asset,
        &payout,
    );
    assert_eq!(client.get_config().source, None);
    assert_eq!(
        client.try_refresh(&admin, &asset),
        Err(Ok(OracleError::MissingPrice.into()))
    );
}

#[test]
fn future_and_nonpositive_votes_are_rejected_before_quorum() {
    let fixture = setup();
    assert_eq!(
        fixture.client.try_submit_fallback(
            &fixture.publisher1,
            &fixture.asset,
            &100_000_000,
            &1_001,
        ),
        Err(Ok(OracleError::FuturePrice.into()))
    );
    assert_eq!(
        fixture
            .client
            .try_submit_fallback(&fixture.publisher1, &fixture.asset, &0, &1_000,),
        Err(Ok(OracleError::InvalidPrice.into()))
    );
}

#[test]
fn configured_feed_cannot_be_written_under_another_asset() {
    let fixture = setup();
    let unrelated = Address::generate(&fixture.env);
    assert_eq!(
        fixture
            .client
            .try_submit_fallback(&fixture.publisher1, &unrelated, &100_000_000, &1_000,),
        Err(Ok(OracleError::UnsupportedAsset.into()))
    );
}

#[test]
fn publisher_rotation_cannot_reuse_votes_from_previous_epoch() {
    let fixture = setup();
    assert!(!fixture.client.submit_fallback(
        &fixture.publisher1,
        &fixture.asset,
        &100_000_000,
        &1_000
    ));
    let replacement = Address::generate(&fixture.env);
    fixture.client.configure_publishers(
        &fixture.admin,
        &fixture.publisher1,
        &fixture.publisher2,
        &replacement,
        &true,
    );
    assert!(!fixture.client.submit_fallback(
        &fixture.publisher2,
        &fixture.asset,
        &100_000_000,
        &1_000
    ));
    assert_eq!(fixture.client.get_latest(&fixture.asset), None);
    assert!(fixture.client.submit_fallback(
        &fixture.publisher1,
        &fixture.asset,
        &100_000_000,
        &1_000
    ));
}

#[test]
fn historical_lookup_excludes_post_expiry_and_stale_observations() {
    let fixture = setup();
    for publisher in [&fixture.publisher1, &fixture.publisher2] {
        fixture
            .client
            .submit_fallback(publisher, &fixture.asset, &100_000_000, &1_000);
    }
    fixture.env.ledger().set_timestamp(1_100);
    for publisher in [&fixture.publisher1, &fixture.publisher2] {
        fixture
            .client
            .submit_fallback(publisher, &fixture.asset, &94_000_000, &1_100);
    }
    assert_eq!(
        fixture
            .client
            .get_price_at_or_before(&fixture.asset, &1_050, &600)
            .unwrap()
            .price,
        100_000_000
    );
    fixture.env.ledger().set_timestamp(1_701);
    assert_eq!(
        fixture
            .client
            .get_price_at_or_before(&fixture.asset, &1_701, &600),
        None
    );
    assert_eq!(fixture.client.get_fresh_price(&fixture.asset), None);
}
