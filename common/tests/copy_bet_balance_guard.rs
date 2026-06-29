//! Funds-sufficiency guard for place_copy_bet, against a real Postgres
//! instance, isolated per run (own schema, dropped on exit).
//!
//! REQUER: container polymarket-bot-postgres-1 rodando (docker-compose up)
//!         + DATABASE_URL no .env.
//! Run: cargo test --manifest-path common/Cargo.toml -- --include-ignored
//! #[ignore] por padrão — sem DB externo o teste é skipped, não falha.
//!
//! place_copy_bet had no funds check at all (not even the rows_affected
//! bookkeeping place_bet has): an under-funded copy-bet still committed,
//! with cost+fee silently NOT deducted (UPDATE matched 0 rows, nobody
//! checked). Same WHERE-clause fix as place_bet (commit 5261191):
//! the bankroll row only matches when value_f64 >= cost+fee, and a new
//! rows_affected() != 2 guard (mirroring place_bet's) rejects + rolls back
//! otherwise.

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
    let schema = format!("copybet_{test_name}_{nonce}");
    let strategy = format!("copybet_strategy_{test_name}_{nonce}");
    let market_id = format!("copybet-market-{test_name}-{nonce}");

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

fn make_copy_bet(strategy: &str, market_id: &str, cost: f64, fee: f64, shares: f64) -> NewBet {
    NewBet {
        market_id: market_id.into(),
        question: "copy-bet balance guard test".into(),
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
        reasoning: "copy-bet e2e".into(),
        end_date: None,
        context: None,
        strategy: strategy.into(),
        source: "copy-bet-e2e".into(),
        url: String::new(),
        event_slug: None,
        features: None,
        copy_ref: None,
        category: None,
    }
}

/// place_copy_bet has no lazy-init of its own — it assumes the strategy's
/// portfolio keys already exist (created by an earlier place_bet call for
/// the same strategy, which is always true for a copy-trading strategy in
/// practice). Seed both keys here to match that precondition.
async fn seed_bankroll(pool: &PgPool, strategy: &str, amount: f64) {
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, $2)")
        .bind(format!("bankroll:{strategy}"))
        .bind(amount)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, 0.0)")
        .bind(format!("signals_sent_today:{strategy}"))
        .execute(pool)
        .await
        .unwrap();
}

async fn read_bankroll(pool: &PgPool, strategy: &str) -> f64 {
    let row: (f64,) = sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
        .bind(format!("bankroll:{strategy}"))
        .fetch_one(pool)
        .await
        .unwrap();
    row.0
}

async fn bet_count(pool: &PgPool, market_id: &str) -> i64 {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bets WHERE market_id = $1")
        .bind(market_id)
        .fetch_one(pool)
        .await
        .unwrap();
    row.0
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn sufficient_balance_copy_bet_succeeds() {
    let (_guard, portfolio, pool, strategy, market_id) = setup("sufficient").await;
    seed_bankroll(&pool, &strategy, 100.0).await;

    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let bet = make_copy_bet(&strategy, &market_id, cost, fee, 20.0);

    let result = portfolio.place_copy_bet(&bet).await;
    assert!(result.is_ok(), "expected Ok, got {result:?}");
    assert_eq!(bet_count(&pool, &market_id).await, 1);
    assert_eq!(read_bankroll(&pool, &strategy).await, 100.0 - (cost + fee));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn insufficient_balance_copy_bet_is_rejected() {
    let (_guard, portfolio, pool, strategy, market_id) = setup("insufficient").await;
    let seeded_bankroll = 5.0_f64;
    seed_bankroll(&pool, &strategy, seeded_bankroll).await;

    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let bet = make_copy_bet(&strategy, &market_id, cost, fee, 20.0);

    let result = portfolio.place_copy_bet(&bet).await;
    assert!(
        result.is_err(),
        "place_copy_bet must reject when bankroll < cost+fee, got {result:?}"
    );
    assert_eq!(
        bet_count(&pool, &market_id).await,
        0,
        "rejected copy-bet must not be persisted"
    );
    assert_eq!(
        read_bankroll(&pool, &strategy).await,
        seeded_bankroll,
        "bankroll must be untouched by a rejected copy-bet"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn burst_of_copy_bets_stops_at_exhausted_bankroll() {
    let (_guard, portfolio, pool, strategy, _market_id) = setup("burst").await;
    seed_bankroll(&pool, &strategy, 25.0).await;

    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let per_bet = cost + fee;

    // 3 copy-bets fired in a row following the same source trader: only
    // the first 2 fit in a 25.0 bankroll (2 * 10.2 = 20.4 <= 25.0 < 30.6).
    let mut results = Vec::new();
    for i in 0..3 {
        let market_id = format!("copybet-burst-market-{i}");
        let bet = make_copy_bet(&strategy, &market_id, cost, fee, 20.0);
        results.push(portfolio.place_copy_bet(&bet).await);
    }

    assert!(results[0].is_ok(), "bet 1 should fit: {:?}", results[0]);
    assert!(results[1].is_ok(), "bet 2 should fit: {:?}", results[1]);
    assert!(
        results[2].is_err(),
        "bet 3 should be rejected (would overdraw): {:?}",
        results[2]
    );

    let bankroll_after = read_bankroll(&pool, &strategy).await;
    let expected = 25.0 - 2.0 * per_bet;
    assert_eq!(bankroll_after, expected);
    assert!(bankroll_after >= 0.0, "bankroll must never go negative");
}
