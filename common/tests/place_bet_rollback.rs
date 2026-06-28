//! Integration test against a real Postgres instance proving that the
//! `rows_affected() != 2` guard in `PostgresStore::place_bet`
//! (common/src/storage/postgres.rs) actually rolls back the whole
//! transaction: no bankroll debit, no orphaned signal counter, no bet row.
//!
//! REQUER: container polymarket-bot-postgres-1 rodando (docker-compose up)
//!         + DATABASE_URL no .env.
//! Run: cargo test --manifest-path common/Cargo.toml -- --include-ignored
//! #[ignore] por padrão — sem DB externo o teste é skipped, não falha.

use chrono::Utc;
use polymarket_common::storage::portfolio::{BetSide, NewBet};
use polymarket_common::storage::postgres::PgPortfolio;
use sqlx::PgPool;

#[tokio::test]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn place_bet_rolls_back_when_portfolio_guard_trips() {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run this integration test");
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to postgres");

    let nonce = Utc::now().timestamp_nanos_opt().unwrap();
    let strategy = format!("test_rollback_{nonce}");
    let market_id = format!("test-market-{nonce}");
    let bankroll_key = format!("bankroll:{strategy}");
    let signals_key = format!("signals_sent_today:{strategy}");
    let trigger_fn = format!("test_drop_signals_row_{nonce}");
    let seeded_bankroll = 200.0_f64;

    cleanup(&pool, &bankroll_key, &signals_key, &market_id, &trigger_fn).await;

    // Pre-seed only the bankroll key (committed, outside place_bet's tx) so the
    // lazy-init INSERT no-ops for it (ON CONFLICT DO NOTHING) and any later
    // change to its value can only come from place_bet's UPDATE.
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, $2)")
        .bind(&bankroll_key)
        .bind(seeded_bankroll)
        .execute(&pool)
        .await
        .unwrap();

    // Fault injection: the signals key does NOT pre-exist, so place_bet's lazy-init
    // INSERT actually creates it inside its transaction. This trigger deletes that
    // row the instant it's inserted, before place_bet's later UPDATE ... WHERE key
    // IN (bankroll_key, signals_key) runs — forcing rows_affected() == 1 and
    // tripping the guard at postgres.rs:885.
    sqlx::query(&format!(
        "CREATE FUNCTION {trigger_fn}() RETURNS TRIGGER AS $$
         BEGIN
           IF NEW.key = '{signals_key}' THEN
             DELETE FROM portfolio WHERE id = NEW.id;
           END IF;
           RETURN NEW;
         END;
         $$ LANGUAGE plpgsql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE TRIGGER {trigger_fn} AFTER INSERT ON portfolio
         FOR EACH ROW EXECUTE FUNCTION {trigger_fn}()"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let store = PgPortfolio::new(pool.clone()).await.unwrap();
    let bet = NewBet {
        market_id: market_id.clone(),
        question: "rollback guard test".into(),
        side: BetSide::Yes,
        entry_price: 0.5,
        slipped_price: 0.5,
        shares: 10.0,
        cost: 5.0,
        fee: 0.1,
        estimated_prob: 0.6,
        confidence: 0.7,
        edge: 0.1,
        kelly_size: 0.05,
        reasoning: "test".into(),
        end_date: None,
        context: None,
        strategy: strategy.clone(),
        source: "test".into(),
        url: String::new(),
        event_slug: None,
        features: None,
        copy_ref: None,
        category: None,
    };

    let result = store.place_bet(&bet).await;

    sqlx::query(&format!("DROP TRIGGER IF EXISTS {trigger_fn} ON portfolio"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(&format!("DROP FUNCTION IF EXISTS {trigger_fn}()"))
        .execute(&pool)
        .await
        .unwrap();

    let err = result.expect_err("place_bet must fail when the portfolio guard trips");
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("expected 2"),
        "unexpected error message: {err_msg}"
    );

    let bankroll_row: (f64,) = sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
        .bind(&bankroll_key)
        .fetch_one(&pool)
        .await
        .expect("bankroll row must still exist");
    assert_eq!(
        bankroll_row.0, seeded_bankroll,
        "bankroll must be unchanged after rollback (no partial debit)"
    );

    let signals_row: Option<(f64,)> =
        sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
            .bind(&signals_key)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(
        signals_row.is_none(),
        "signals counter must not survive rollback, got {signals_row:?}"
    );

    let bet_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bets WHERE market_id = $1")
        .bind(&market_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(bet_count.0, 0, "no bet row must be persisted after rollback");

    cleanup(&pool, &bankroll_key, &signals_key, &market_id, &trigger_fn).await;
}

async fn cleanup(
    pool: &PgPool,
    bankroll_key: &str,
    signals_key: &str,
    market_id: &str,
    trigger_fn: &str,
) {
    sqlx::query(&format!("DROP TRIGGER IF EXISTS {trigger_fn} ON portfolio"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(&format!("DROP FUNCTION IF EXISTS {trigger_fn}()"))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM portfolio WHERE key IN ($1, $2)")
        .bind(bankroll_key)
        .bind(signals_key)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM bets WHERE market_id = $1")
        .bind(market_id)
        .execute(pool)
        .await
        .unwrap();
}
