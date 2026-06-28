//! End-to-end test: signal -> place_bet -> portfolio -> settle, against a
//! real Postgres instance, isolated in its own schema (created/dropped per run).
//! Only the Polymarket API boundary is mocked (price feed stub) — place_bet
//! and resolve_bet (common/src/storage/postgres.rs) run unmodified.
//!
//! REQUER: container polymarket-bot-postgres-1 rodando (docker-compose up)
//!         + DATABASE_URL no .env.
//! Run: cargo test --manifest-path common/Cargo.toml -- --include-ignored
//! #[ignore] por padrão — sem DB externo o teste é skipped, não falha.

use chrono::Utc;
use polymarket_common::storage::portfolio::{BetSide, NewBet};
use polymarket_common::storage::postgres::PgPortfolio;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Stand-in for the Polymarket price feed — the only externally-facing
/// dependency in the signal -> bet flow. Kept as a trait so the mock is
/// explicit at the API boundary, not woven into place_bet/resolve_bet.
trait MarketPriceFeed {
    fn current_price(&self, market_id: &str) -> f64;
}

struct MockPolymarketApi;

impl MarketPriceFeed for MockPolymarketApi {
    fn current_price(&self, _market_id: &str) -> f64 {
        0.5
    }
}

/// Drops the isolated test schema on scope exit, including on panic/assert
/// failure, so a failed assertion never leaks schemas into the shared DB.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres (DATABASE_URL)"]
async fn signal_to_place_bet_to_settle_e2e() {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run this integration test");

    let nonce = Utc::now().timestamp_nanos_opt().unwrap();
    let schema = format!("e2e_{nonce}");
    let strategy = format!("e2e_strategy_{nonce}");
    let market_id = format!("e2e-market-{nonce}");
    let bankroll_key = format!("bankroll:{strategy}");

    let root_pool = PgPool::connect(&url).await.expect("connect to postgres");
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&root_pool)
        .await
        .expect("create isolated schema");

    let _guard = SchemaGuard {
        root_pool: root_pool.clone(),
        schema: schema.clone(),
    };

    // Every connection in this pool is pinned to the isolated schema, so
    // migrations and all queries below touch nothing outside e2e_<nonce>.
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

    let seeded_bankroll = 1000.0_f64;
    sqlx::query("INSERT INTO portfolio (key, value_f64) VALUES ($1, $2)")
        .bind(&bankroll_key)
        .bind(seeded_bankroll)
        .execute(&isolated_pool)
        .await
        .unwrap();

    // --- 1. signal IN: a market price is fetched at the API boundary and
    // turned into a concrete bet decision (sizing logic itself is out of
    // scope here; the point is exercising the real persistence path).
    let api = MockPolymarketApi;
    let market_price = api.current_price(&market_id);
    let cost = 10.0_f64;
    let fee = 0.2_f64;
    let shares = 20.0_f64;

    let bet = NewBet {
        market_id: market_id.clone(),
        question: "e2e signal test".into(),
        side: BetSide::Yes,
        entry_price: market_price,
        slipped_price: market_price,
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
        strategy: strategy.clone(),
        source: "e2e".into(),
        url: String::new(),
        event_slug: None,
        features: None,
        copy_ref: None,
        category: None,
    };

    // --- 2. place_bet (real): persists bet + portfolio, decrements bankroll.
    let bet_id = portfolio
        .place_bet(&bet)
        .await
        .expect("place_bet should succeed");

    let expected_after_place = seeded_bankroll - (cost + fee);

    // --- 3. assert OPEN state + bankroll debit + portfolio row.
    let (resolved, won, pnl): (bool, Option<bool>, Option<f64>) =
        sqlx::query_as("SELECT resolved, won, pnl FROM bets WHERE id = $1")
            .bind(bet_id)
            .fetch_one(&isolated_pool)
            .await
            .unwrap();
    assert!(!resolved, "bet must be OPEN right after place_bet");
    assert_eq!(won, None);
    assert_eq!(pnl, None);

    let bankroll_after_place: (f64,) =
        sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
            .bind(&bankroll_key)
            .fetch_one(&isolated_pool)
            .await
            .unwrap();
    assert_eq!(
        bankroll_after_place.0, expected_after_place,
        "bankroll must be exactly seeded - (cost + fee) after place_bet"
    );

    // --- 4. settle market WIN (real resolve_bet) -> payout, bankroll credited.
    let resolved_bet = portfolio
        .resolve_bet(&market_id, true)
        .await
        .expect("resolve_bet should succeed")
        .expect("resolve_bet must find the open bet");

    // Mirror resolve_bet's own arithmetic (postgres.rs ~1043-1047) so the
    // expected value is bit-identical, not an approximation.
    let fee_pct = 0.02_f64;
    let gross_payout = shares; // side=Yes, yes_won=true => bet_won=true
    let exit_fee = gross_payout * fee_pct;
    let net_payout = gross_payout - exit_fee;
    let expected_pnl = net_payout - cost - fee;
    let expected_after_settle = expected_after_place + net_payout;

    assert!(resolved_bet.won);
    assert_eq!(resolved_bet.pnl, expected_pnl);

    // --- 5. assert SETTLED state, no OPEN bets left, exact final bankroll.
    let (resolved, won, pnl): (bool, Option<bool>, Option<f64>) =
        sqlx::query_as("SELECT resolved, won, pnl FROM bets WHERE id = $1")
            .bind(bet_id)
            .fetch_one(&isolated_pool)
            .await
            .unwrap();
    assert!(resolved, "bet must be SETTLED after resolve_bet");
    assert_eq!(won, Some(true));
    assert_eq!(pnl, Some(expected_pnl));

    let open_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM bets WHERE market_id = $1 AND resolved = false")
            .bind(&market_id)
            .fetch_one(&isolated_pool)
            .await
            .unwrap();
    assert_eq!(open_count.0, 0, "no OPEN bets must remain for this market");

    let bankroll_after_settle: (f64,) =
        sqlx::query_as("SELECT value_f64 FROM portfolio WHERE key = $1")
            .bind(&bankroll_key)
            .fetch_one(&isolated_pool)
            .await
            .unwrap();
    assert_eq!(
        bankroll_after_settle.0, expected_after_settle,
        "bankroll must be exactly credited with net_payout after settle"
    );
}
