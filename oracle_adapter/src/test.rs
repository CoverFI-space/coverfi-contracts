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
    client.initialize(&admin, &source, &Symbol::new(&env, "XLM"), &8, &600, &2_000);
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
