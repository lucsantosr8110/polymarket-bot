//! E2E error-path tests against a real Postgres instance, isolated per run
//! (own schema, dropped on exit). place_bet/resolve_bet run unmodified —
//! see common/src/storage/postgres.rs.
//!
//! REQUER: container polymarket-bot-postgres-1 rodando (docker-compose up)
//!         + DATABASE_URL no .env.
//! Run: cargo test --manifest-path common/Cargo.toml -- --include-ignored
//! #[ignore] por padrão — sem DB externo o teste é skipped, não falha.
//!
//! T1/T1b cover the funds-sufficiency guard in place_bet: the UPDATE only
//! matches the bankroll row when value_f64 >= cost+fee, so the existing
//! rows_affected == 2 check at postgres.rs:885 rejects (rolls back) any
//! bet that would take the bankroll negative.

use chrono::Utc;
use polymarket_common::storage::portfolio::{BetSide, NewBet};
use polymarket_common::storage::postgres::PgPortfolio;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

struct SchemaGuard {
    root_pool: PgPool,
    schema: String,
}

impl Drop for SchemaGuard {
    fn drop(&mut self) {
        let root_pool = self.root_pool.clone();
        let schema = self.schema.clone();
        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(move || {
            handle.block_on(async move {
                let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
                    .execute(&root_pool)
                    .await;
            });
        });
    }
}

/// Spins up an isolated schema, runs migrations into it, and hands back a
/// ready-to-use portfolio store plus a unique strategy/market namespace.
async fn setup(test_name: &str) -> (SchemaGuard, PgPortfolio, PgPool, String, String) {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run this integration test");

    let nonce = Utc::now().timestamp_nanos_opt().unwrap();
    let schema = format!("e2e_{test_name}_{nonce}");
    let strategy = format!("e2e_strategy_{test_name}_{nonce}");
    let market_id = format!("e2e-market-{test_name}-{nonce}");

    let root_pool = PgPool::connect(&url).await.expect("connect to postgres");
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&root_pool)
        .await
        .expect("create isolated schema");

    let guard = SchemaGuard {
        root_pool: root_pool.clone(),
        schema: schema.clone(),
    };

    let isolated_pool = PgPoolOptions::new()
        .max_connections(5)
        .after_connect({
            let schema = schema.clone();
            move |conn, _meta| {
                let schema = schema.clone();
                Box::pin(async move {
                    sqlx::query(&format!("SET search_path TO \"{schema}\""))
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            }
        })
        .connect(&url)
        .await
        .expect("connect isolated pool");

    let portfolio = PgPortfolio::new(isolated_pool.clone())
        .await
        .expect("construct PgPortfolio");
    portfolio
        .run_migrations()
        .await
        .expect("run migrations in isolated schema");

    (guard, portfolio, isolated_pool, strategy, market_id)
}

fn make_bet(strategy: &str, market_id: &str, cost: f64, fee: f64, shares: f64) -> NewBet {
    NewBet {
        market_id: market_id.into(),
        question: "e2e error path test".into(),
        side: BetSide::Yes,
        entry_price: 0.5,
        slipped_price: 0.5,
        shares,
        cost,
        fee,
        estimated_prob: 0.65,
        confidence: 0.8,
        edge: 0.15,
        kelly_size: 0.05,
        reasoning: "e2e".into(),
        end_date: None,
        context: None,
        strategy: strategy.into(),
        source: "e2e".into(),
        url: String::new(),
        event_slug: None,
        features: None,
        copy_ref: None,
        category: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn t1_insufficient_bankroll_is_rejected() {
    let (_guard, portfolio, pool, strategy, market_id) = setup("t1").await;
    let bankroll_key = format!("bankroll:{strategy}");

    let seeded_bankroll = 5.0_f64;
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, $2)")
        .bind(&bankroll_key)
        .bind(seeded_bankroll)
        .execute(&pool)
        .await
        .unwrap();

    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let bet = make_bet(&strategy, &market_id, cost, fee, 20.0);

    let result = portfolio.place_bet(&bet).await;
    assert!(
        result.is_err(),
        "place_bet must reject when bankroll < cost+fee, got {result:?}"
    );

    let bet_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bets WHERE market_id = $1")
        .bind(&market_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(bet_count.0, 0, "rejected bet must not be persisted");

    let bankroll_after: (f64,) = sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
        .bind(&bankroll_key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        bankroll_after.0, seeded_bankroll,
        "bankroll must be untouched by a rejected bet"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn t1b_exact_balance_bet_is_accepted() {
    let (_guard, portfolio, pool, strategy, market_id) = setup("t1b").await;
    let bankroll_key = format!("bankroll:{strategy}");

    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let seeded_bankroll = cost + fee;
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, $2)")
        .bind(&bankroll_key)
        .bind(seeded_bankroll)
        .execute(&pool)
        .await
        .unwrap();

    let bet = make_bet(&strategy, &market_id, cost, fee, 20.0);
    let result = portfolio.place_bet(&bet).await;
    assert!(
        result.is_ok(),
        "place_bet must accept a bet that exactly exhausts the bankroll, got {result:?}"
    );

    let bankroll_after: (f64,) = sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
        .bind(&bankroll_key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(bankroll_after.0, 0.0, "bankroll must land exactly at zero");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn t2_settle_loss_debits_no_payout() {
    let (_guard, portfolio, pool, strategy, market_id) = setup("t2").await;
    let bankroll_key = format!("bankroll:{strategy}");

    let seeded_bankroll = 1000.0_f64;
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, $2)")
        .bind(&bankroll_key)
        .bind(seeded_bankroll)
        .execute(&pool)
        .await
        .unwrap();

    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let bet = make_bet(&strategy, &market_id, cost, fee, 20.0);
    let bet_id = portfolio.place_bet(&bet).await.expect("place_bet ok");

    let bankroll_after_place = seeded_bankroll - (cost + fee); // 989.8

    let resolved_bet = portfolio
        .resolve_bet(&market_id, false) // side=Yes, yes_won=false => bet lost
        .await
        .expect("resolve_bet ok")
        .expect("must find the open bet");

    let expected_pnl = -cost - fee; // gross_payout=0 when lost -> net_payout=0
    assert!(!resolved_bet.won);
    assert_eq!(resolved_bet.pnl, expected_pnl);

    let (resolved, won, pnl): (bool, Option<bool>, Option<f64>) =
        sqlx::query_as("SELECT resolved, won, pnl FROM bets WHERE id = $1")
            .bind(bet_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(resolved);
    assert_eq!(won, Some(false));
    assert_eq!(pnl, Some(expected_pnl));

    let bankroll_after_settle: (f64,) =
        sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
            .bind(&bankroll_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        bankroll_after_settle.0, bankroll_after_place,
        "no payout on loss; bankroll stays at post-place value"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn t3_resolve_bet_is_idempotent() {
    let (_guard, portfolio, pool, strategy, market_id) = setup("t3").await;
    let bankroll_key = format!("bankroll:{strategy}");

    let seeded_bankroll = 1000.0_f64;
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, $2)")
        .bind(&bankroll_key)
        .bind(seeded_bankroll)
        .execute(&pool)
        .await
        .unwrap();

    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let shares = 20.0_f64;
    let bet = make_bet(&strategy, &market_id, cost, fee, shares);
    portfolio.place_bet(&bet).await.expect("place_bet ok");

    let first = portfolio
        .resolve_bet(&market_id, true)
        .await
        .expect("first resolve ok")
        .expect("must resolve the open bet");

    let fee_pct = 0.02_f64;
    let gross_payout = shares;
    let exit_fee = gross_payout * fee_pct;
    let net_payout = gross_payout - exit_fee;
    let expected_pnl = net_payout - cost - fee;
    let expected_bankroll = seeded_bankroll - (cost + fee) + net_payout; // 1009.4

    assert_eq!(first.pnl, expected_pnl);

    let bankroll_after_first: (f64,) =
        sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
            .bind(&bankroll_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(bankroll_after_first.0, expected_bankroll);

    // Second resolve on the same (already-resolved) market must be a no-op:
    // the lookup query filters `resolved = false`, so it finds nothing.
    let second = portfolio
        .resolve_bet(&market_id, true)
        .await
        .expect("second resolve call must not error");
    assert!(
        second.is_none(),
        "resolve_bet must not re-resolve an already-settled bet"
    );

    let bankroll_after_second: (f64,) =
        sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
            .bind(&bankroll_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        bankroll_after_second.0, expected_bankroll,
        "bankroll must not change on the second, idempotent resolve call"
    );

    let (won, pnl): (Option<bool>, Option<f64>) =
        sqlx::query_as("SELECT won, pnl FROM bets WHERE market_id = $1")
            .bind(&market_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(won, Some(true));
    assert_eq!(pnl, Some(expected_pnl));
}
