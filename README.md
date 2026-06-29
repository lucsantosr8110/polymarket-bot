# Polymarket Signal Bot

> **Research project only.** This bot is built for learning and experimentation. It is not intended for production use, and you should not risk real money with it. Use at your own risk.

A Cargo workspace containing two independent Rust trading bots for Polymarket prediction markets, backed by a shared library, a Python ML ensemble, and PostgreSQL.

- **trading-bot** — ML-driven signals (XGBoost ensemble + LLM consensus + Bayesian updating)
- **copy-trading-bot** — mirrors trades from top leaderboard traders

Both bots run independently with their own Telegram interfaces, and share the same database and common library.

## Architecture

```mermaid
graph TB
    subgraph Workspace["Cargo Workspace"]
        Common["polymarket-common<br/>(shared lib)"]
        TB["trading-bot<br/>(bin)"]
        CTB["copy-trading-bot<br/>(bin)"]
        TB --> Common
        CTB --> Common
    end

    subgraph External["External Services"]
        Gamma["Polymarket<br/>Gamma API"]
        CLOB["Polymarket<br/>CLOB API"]
        WS["Polymarket<br/>WebSocket"]
        News["News RSS<br/>Feeds"]
        OpenAI["OpenAI API"]
    end

    subgraph Infra["Infrastructure"]
        PG[(PostgreSQL)]
        Sidecar["Model Server<br/>(FastAPI)"]
        Prom["Prometheus"]
        Graf["Grafana"]
    end

    TB --> Gamma & CLOB & WS & News & OpenAI
    TB --> PG & Sidecar
    CTB --> Gamma & CLOB
    CTB --> PG
    TB -.->|metrics :9000| Prom
    CTB -.->|metrics :9001| Prom
    Prom --> Graf
```

### Crate Structure

```
polymarket-bot/                  # Cargo workspace root
├── common/src/                  # Shared library (polymarket-common)
│   ├── data/
│   │   ├── models.rs            # GammaMarket, PriceTick data structures
│   │   └── crawler.rs           # Historical data crawler for backtesting
│   ├── model/
│   │   └── features.rs          # MarketFeatures (v5, 29 features), feature engineering
│   ├── pricing/
│   │   └── kelly.rs             # Kelly Criterion position sizing
│   ├── storage/
│   │   ├── portfolio.rs         # Bet, NewBet, BetContext, CopyRef domain types
│   │   ├── postgres.rs          # PostgreSQL persistence layer
│   │   └── copy_trade.rs        # Copy-trading DB operations
│   ├── telegram/
│   │   └── notifier.rs          # Telegram send/broadcast, polling, subscribers
│   ├── signal.rs                # SignalSource enum (XgBoost, LlmConsensus, CopyTrade)
│   ├── format.rs                # Message formatting helpers
│   └── metrics.rs               # Prometheus + tokio runtime metrics
│
├── trading-bot/src/             # ML trading bot
│   ├── main.rs                  # Entry point, multi-loop orchestration
│   ├── config.rs                # AppConfig (env vars via confique)
│   ├── strategy.rs              # Strategy profiles (aggressive/balanced/conservative)
│   ├── bayesian.rs              # Bayesian updating with likelihood ratios
│   ├── calibration.rs           # Calibration curve from resolved LLM estimates
│   ├── scanner/
│   │   ├── live.rs              # LiveScanner: Gamma fetch → XGBoost → Bayesian → Signal
│   │   ├── ws.rs                # WebSocket price/activity alerts
│   │   └── news.rs              # Multi-source news + embedding similarity matching
│   ├── cycles/
│   │   ├── bet_scan.rs          # Signal → Kelly sizing → place bet → log
│   │   ├── housekeeping.rs      # Resolve bets, stop-loss, expiry exits, calibration
│   │   ├── alerts.rs            # WS-triggered market reassessment
│   │   └── heartbeat.rs         # Periodic status reports
│   ├── model/
│   │   ├── xgb.rs               # Pure-Rust XGBoost inference (JSON tree traversal)
│   │   └── sidecar.rs           # HTTP client for Python ensemble server
│   ├── backtest/
│   │   ├── engine.rs            # Backtest runner with pluggable strategies
│   │   ├── metrics.rs           # Sharpe, Brier score, max drawdown
│   │   └── portfolio.rs         # In-memory portfolio for backtesting
│   └── telegram/
│       └── commands.rs          # /stats, /open, /brier, /health, /help
│
├── copy-trading-bot/src/        # Copy trading bot
│   ├── main.rs                  # Entry point, event loop
│   ├── config.rs                # CopyTradingConfig (minimal env vars)
│   ├── scanner/
│   │   └── copy_trader.rs       # Leaderboard polling, trade detection, staleness filter
│   ├── cycles/
│   │   ├── copy_trade.rs        # Detect → Kelly size → place copy bet → log
│   │   └── housekeeping.rs      # Resolve copy trades, send results
│   └── telegram/
│       └── commands.rs          # /stats, /copy, /traders, /leaderboard, /follow, /unfollow
│
├── scripts/                     # Python ML pipeline
│   ├── fetch_data.py            # Fetch resolved markets + price history → training_data.json
│   ├── train_model.py           # Train XGBoost/stacking ensemble, export JSON for Rust
│   └── serve_model.py           # FastAPI sidecar: /predict, /predict_batch, /retrain, /health
│
├── migrations/                  # SQLx PostgreSQL migrations (20 files)
├── model/                       # Trained model artifacts (JSON, joblib)
├── monitoring/                  # Prometheus + Grafana configs
└── adr/                         # Architecture Decision Records
```

### Migrations convention

`migrations/` holds only schema (`CREATE`/`ALTER`) and idempotent declarative
seed data. Manual runtime ops (backfills, one-off fixes, data corrections)
do not belong here — they pollute schema history and re-run on every fresh
DB. Applied migrations are immutable (sqlx checksums them): never edit or
delete a migration that has run anywhere.

### Runtime dashboard config

The dashboard writes live settings to PostgreSQL table `runtime_config`.
`trading-bot` polls that table with `CONFIG_POLL_INTERVAL_SECS` (default `60`)
and swaps an in-memory snapshot without restart.

`CONFIG_POLL_INTERVAL_SECS` (env) only takes effect on a fresh database
before `runtime_config.global` has a seeded row — migration `022` seeds
`config_poll_interval_secs` explicitly, and the DB value always wins over
the env var once that row exists (`RuntimeGlobals::overlay`). To change
poll cadence on a running deployment, edit it via the dashboard
(Config Global → `config_poll_interval_secs`) or update the DB row
directly, not the env var.

Runtime-applied fields currently include:
`strategies`, `active_strategies`, `scan_interval_mins`,
`bet_scan_interval_mins`, `heartbeat_interval_mins`,
`config_poll_interval_secs`, `stop_loss_pct`,
`exit_days_before_expiry`, `slippage_pct`, `min_kelly_size`,
`min_bet_price`, `max_ws_bets_per_day`, `alert_throttle_mins`,
`ws_bet_cooldown_secs`, and
`price_alert_cooldown_secs`.

Fields not listed above remain startup-only until wired into the Rust runtime.
`take_profit_pct` is stored for dashboard planning but is not yet enforced by
the bot. Strategy Kelly sizing is runtime-editable per strategy; the global
`kelly_fraction` field is kept for dashboard compatibility only.

Validate a live reload with:

```bash
DATABASE_URL=postgres://bot:bot@localhost:5432/polymarket \
DASHBOARD_API_URL=http://localhost:8001 \
scripts/test_runtime_config.sh Balanced 0.30
```

## How It Works

### Trading Bot — ML Pipeline

```mermaid
flowchart LR
    A[Gamma API<br/>fetch markets] --> B[Filter<br/>volume / expiry / price]
    B --> C[Feature Engineering<br/>29 features v5]
    C --> D[XGBoost Ensemble<br/>local or sidecar]
    D --> E[Bayesian Anchoring<br/>LR^conf × damping]
    E --> F[Signal Filters<br/>sports / YES block]
    F --> G[Correlation Check<br/>LLM dedup]
    G --> H[Strategy Eval<br/>per-profile thresholds]
    H --> I[Kelly Sizing<br/>fractional Kelly]
    I --> J[Place Bet<br/>+ Telegram alert]
```

1. **Market fetch** — fetches eligible markets from Gamma API (volume, expiry, price filters)
2. **Feature engineering** — computes 29 features per market (v5): price momentum, volatility, RSI, order book stats, temporal features, 16 NLP features from question text
3. **XGBoost ensemble** — scores markets via local pure-Rust XGBoost or Python sidecar (XGBoost + LightGBM + HistGBM + ExtraTrees + RF + meta-learner)
4. **Bayesian anchoring** — model predictions anchored to market price as prior; likelihood ratio dampened by `LR^(confidence × 0.5)` to prevent overconfidence
5. **Signal filters** (ADR 009) — blocks sports/esports markets and YES-side XGBoost bets based on profitability analysis
6. **Correlation check** (ADR 007) — LLM call detects correlated/mutually-exclusive bets against open positions (fail-open)
7. **Strategy evaluation** — each profile independently checks edge/confidence thresholds
8. **Kelly sizing** — fractional Kelly with terminal risk scaling and min-bet gates
9. **Bet placement** — paper bet logged to DB with full feature snapshot; Telegram notification sent

### Copy Trading Bot

```mermaid
flowchart LR
    A[Polymarket<br/>Leaderboard API] --> B[Detect New Trades<br/>from followed traders]
    B --> C[Staleness Filter<br/>price drift check]
    C --> D[Kelly Sizing]
    D --> E[Place Copy Bet<br/>with CopyRef]
    E --> F[Telegram Alert]
```

Polls top traders on the Polymarket leaderboard every minute. When a followed trader opens a new position, the bot mirrors it after filtering for staleness and price drift.

### WebSocket Alerts

A parallel WebSocket connection monitors real-time price movements. Significant moves (3%+) trigger instant reassessment through XGBoost. Open bet price alerts use a higher threshold (5%+ with 1h cooldown) to reduce noise.

### Continuous Learning

The model sidecar retrains every 24 hours on:
- **~3000 resolved Polymarket markets** fetched from the API
- **Own resolved bets** (weighted 3x) with exact entry prices and known outcomes

### Risk Management

- **Signal filters**: blocks unprofitable segments (sports, YES-side) based on live performance analysis
- **Portfolio correlation**: LLM-based detection of correlated/mutually-exclusive bets
- **Stop-loss** (default disabled): exits positions when unrealized loss exceeds threshold
- **Expiry exit** (default disabled): exits underwater positions approaching expiry
- **Terminal risk scaling**: reduces position size as market approaches expiry
- **Per-strategy bankrolls**: each strategy has independent bankroll isolation
- **Min Kelly gate**: signals below 2% Kelly fraction are filtered out
- **Min bet price**: entry side must be >= 0.15 to avoid near-certain markets
- **Graceful shutdown**: handles SIGTERM/SIGINT, sends Telegram notification before exit

### Housekeeping Loop

A separate loop (every 30 min) resolves settled bets, checks stop-loss/expiry exits, updates calibration data, monitors model retrain freshness, logs predictions for Brier score tracking, and sends daily performance reports.

## Strategy Profiles

Three strategies run simultaneously with independent bankrolls (default €300 each):

| Strategy | Kelly | Min Edge | Min Confidence | Max Signals/Day | Min Bet |
|---|---|---|---|---|---|
| **Aggressive** | 50% | 5% | 40% | 10 | €5 |
| **Balanced** | 25% | 6% | 40% | 5 | €5 |
| **Conservative** | 15% | 8% | 50% | 3 | €15 |

## Telegram Commands

### Trading Bot

| Command | Description |
|---|---|
| `/stats` | Portfolio statistics with per-strategy breakdown |
| `/open` | Open positions with live prices, PnL, Polymarket links |
| `/brier` | Model accuracy per source (Brier score vs market baseline) |
| `/health` | Bot uptime, scan counts, signals found |
| `/help` | List commands |

### Copy Trading Bot

| Command | Description |
|---|---|
| `/stats` | Copy trading results |
| `/copy` | Open copy-trade positions |
| `/traders` | Followed traders list |
| `/leaderboard` | Top Polymarket traders (day / month / all-time) |
| `/follow <wallet>` | Follow a trader (owner only) |
| `/unfollow <wallet>` | Unfollow a trader (owner only) |
| `/help` | List commands |

### Notifications

- **New bet**: strategy, side, stake, edge, confidence, reasoning
- **Bet resolved**: outcome, PnL, per-strategy record + random victory GIF on wins
- **Price moves**: alerts on 5%+ moves on open bets (1h cooldown per market)
- **WS-triggered bets**: real-time bets from WebSocket price alerts
- **Heartbeat**: hourly summary with open bets, scan stats, strategy performance
- **Daily report**: full portfolio breakdown with Brier score

## Setup

```bash
cp .env.example .env
# Edit .env with your credentials
```

### Docker Compose (recommended)

```bash
docker compose up -d        # add --build to rebuild local images (dashboard-*, model-server)
```

Starts 8 services:

| Service | Port | Description |
|---|---|---|
| **postgres** | 5432 | PostgreSQL 17 — shared data persistence |
| **model-server** | — | Python FastAPI sidecar — ensemble predictions + scheduled retraining |
| **bot** | 9000 | Trading bot — pre-built image from GHCR, Prometheus metrics on 9000 |
| **copy-trading-bot** | 9001 | Copy trading bot — pre-built image from GHCR, metrics on 9001 |
| **dashboard-api** | 8001 | FastAPI backend for the dashboard (built locally) |
| **dashboard-ui** | 5173 | React dashboard served by nginx (built locally) |
| **prometheus** | 9090 | Metrics collection (scrapes both bots) |
| **grafana** | 3000 | Dashboards |

### Access points

| URL | What |
|---|---|
| http://localhost:5173 | **Dashboard** — KPIs, bets, strategies, live config, logs |
| http://localhost:8001/api/health | Dashboard API health probe |
| http://localhost:3000 | **Grafana** (default `admin` / `admin` — change in prod) |
| http://localhost:9090 | Prometheus |

The dashboard UI is served by nginx, which reverse-proxies `/api/*` (REST + the
logs WebSocket) to `dashboard-api`, so the SPA stays same-origin — no CORS and
the API need not be exposed publicly. Editing a strategy or global field in the
dashboard writes to the `runtime_config` table; the bot picks it up on its next
polling cycle (`CONFIG_POLL_INTERVAL_SECS`, default 60s) — watch for a
`Runtime config reloaded from database` log line.

### Production deploy

Deploy is automated by `.github/workflows/release.yml` (version bump → build &
push bot images to GHCR → `deploy.yml` over SSH to the Hetzner host). The deploy
step rsyncs `docker-compose.yml`, `scripts/`, `migrations/`, `monitoring/`,
`dashboard_api/`, and `dashboard-ui/`, then on the server runs:

```bash
docker compose build model-server dashboard-api dashboard-ui
docker compose up -d
```

Secrets never live in the repo: the server `.env` is written from the
`DEPLOY_ENV` GitHub secret, and `.env` is git-ignored. For production also
override the Grafana admin credentials (`GF_SECURITY_ADMIN_PASSWORD`) and the
Postgres password via the server environment.

### Running locally without the private GHCR images

The `bot`/`copy-trading-bot` images default to the private GHCR registry. To run
the full stack locally without registry access, build glibc images from source
and tag them with the names the compose file expects:

```bash
cargo build -p trading-bot -p copy-trading-bot --release
docker build -f trading-bot/Dockerfile.local      -t ghcr.io/skharchikov/polymarket-bot:latest .
docker build -f copy-trading-bot/Dockerfile.local -t ghcr.io/skharchikov/polymarket-bot-copy:latest .

mkdir -p logs && chmod 777 logs   # bot runs as uid 10001 and tees to logs/bot.log

# copy-trading-bot reuses the main Telegram creds when its own are unset:
export COPY_TELEGRAM_BOT_TOKEN="$(grep -E '^TELEGRAM_BOT_TOKEN=' .env | cut -d= -f2-)"
export COPY_TELEGRAM_CHAT_ID="$(grep -E '^TELEGRAM_CHAT_ID=' .env | cut -d= -f2-)"

docker compose up -d
```

> The `model` volume starts empty, so `model-server` trains from scratch on first
> boot. To use an already-trained `model/` directory, seed the volume first:
> `docker run --rm -v polymarket-bot_model:/model -v "$(pwd)/model":/src:ro alpine cp -a /src/. /model/`

### Local Development

```bash
# Start Postgres
docker run -d \
  -e POSTGRES_DB=polymarket \
  -e POSTGRES_USER=bot \
  -e POSTGRES_PASSWORD=bot \
  -p 5432:5432 \
  postgres:17-alpine

# Trading bot
cargo run -p trading-bot             # Live mode
cargo run -p trading-bot -- test     # Test mode (2-min intervals)
cargo run -p trading-bot -- backtest # Backtest on historical data

# Copy trading bot
cargo run -p copy-trading-bot
```

## Configuration

### Trading Bot

All settings via environment variables with sensible defaults. Trading fees
are taken from each market's live `feeSchedule` (returned by the Gamma API);
the `FEE_PCT_*` vars below are only the fallback used when a market has no
schedule:

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | *required* | Postgres connection string |
| `TELEGRAM_BOT_TOKEN` | *required* | Telegram bot token |
| `TELEGRAM_CHAT_ID` | *required* | Telegram chat ID (owner) |
| `MODEL_SIDECAR_URL` | `` | ML model server URL (auto in Docker) |
| `LLM_MODEL` | `gpt-4o` | LLM model for news assessment fallback |
| `STRATEGIES` | `aggressive,balanced,conservative` | Active strategy profiles |
| `STRATEGY_BANKROLL` | `300.0` | Starting bankroll per strategy (EUR) |
| `SCAN_INTERVAL_MINS` | `30` | Housekeeping loop interval |
| `NEWS_SCAN_INTERVAL_MINS` | `10` | News scan loop interval |
| `BET_SCAN_INTERVAL_MINS` | `10` | Bet scan loop interval |
| `NEWS_ENABLED` | `false` | Enable news fetching + embedding matching |
| `MAX_MODEL_CANDIDATES` | `15` | Top N markets from XGBoost ranking |
| `MAX_LLM_CANDIDATES` | `1` | Markets assessed by LLM per cycle |
| `MIN_VOLUME` | `1000.0` | Min market volume filter |
| `MIN_BOOK_DEPTH` | `200.0` | Min order book depth (USD) |
| `MAX_DAYS_TO_EXPIRY` | `14` | Max days to market expiry |
| `KELLY_FRACTION` | `0.25` | Global Kelly fraction |
| `MIN_EFFECTIVE_EDGE` | `0.08` | Global min effective edge |
| `SLIPPAGE_PCT` | `0.01` | Slippage assumption (1%) |
| `FEE_PCT_DEFAULT` | `0.0` | Fallback fee for markets with no category (e.g. geopolitics) |
| `FEE_PCT_CRYPTO` | `0.018` | Fallback fee for Crypto-category markets |
| `FEE_PCT_SPORTS` | `0.0075` | Fallback fee for Sports-category markets |
| `FEE_PCT_POLITICS` | `0.01` | Fallback fee for Politics-category markets |
| `FEE_PCT_FINANCE` | `0.01` | Fallback fee for Finance/Business/Economics-category markets |
| `FEE_PCT_OTHER` | `0.0125` | Fallback fee for any other recognized category |
| `STOP_LOSS_PCT` | `999.0` | Stop-loss threshold (999.0 = disabled) |
| `EXIT_DAYS_BEFORE_EXPIRY` | `0` | Exit underwater positions N days before expiry (0 = disabled) |
| `BLOCK_SPORTS` | `true` | Block sports/esports markets |
| `BLOCK_YES_SIDE` | `true` | Block YES-side XGBoost bets |
| `LR_DAMPING` | `0.5` | Bayesian LR damping multiplier |
| `MIN_KELLY_SIZE` | `0.02` | Min Kelly fraction to emit signal |
| `MIN_BET_PRICE` | `0.15` | Min entry-side price |
| `CONSENSUS_AGENTS` | `2` | Number of LLM agents for fallback (1-3) |
| `HEARTBEAT_INTERVAL_MINS` | `60` | Heartbeat interval (0 = disabled) |
| `RETRAIN_INTERVAL_HOURS` | `24` | Expected model retrain interval |
| `METRICS_PORT` | `9000` | Prometheus metrics port |

### Copy Trading Bot

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | *required* | Postgres connection string |
| `TELEGRAM_BOT_TOKEN` | *required* | Telegram bot token (separate from trading bot) |
| `TELEGRAM_CHAT_ID` | *required* | Telegram chat ID |
| `COPY_TRADE_INTERVAL_MINS` | `1` | Poll interval for new trades |
| `SLIPPAGE_PCT` | `0.01` | Slippage assumption (1%) |
| `FEE_PCT_DEFAULT` | `0.0` | Fallback fee for markets with no category (shared with trading bot) |
| `FEE_PCT_CRYPTO` | `0.018` | Fallback fee for Crypto-category markets |
| `FEE_PCT_SPORTS` | `0.0075` | Fallback fee for Sports-category markets |
| `FEE_PCT_POLITICS` | `0.01` | Fallback fee for Politics-category markets |
| `FEE_PCT_FINANCE` | `0.01` | Fallback fee for Finance/Business/Economics-category markets |
| `FEE_PCT_OTHER` | `0.0125` | Fallback fee for any other recognized category |
| `METRICS_PORT` | `9001` | Prometheus metrics port |

## ML Model

### Feature Vector (v5 — 29 features)

| # | Feature | Source |
|---|---|---|
| 1-5 | `yes_price`, `momentum_1h`, `momentum_24h`, `volatility_24h`, `rsi` | CLOB price history |
| 6-7 | `log_volume`, `days_to_expiry` | Gamma API |
| 8 | `is_crypto` | Category keyword matching |
| 9-10 | `price_change_1d`, `price_change_1w` | Gamma API price changes |
| 11-12 | `days_since_created`, `created_to_expiry_span` | Temporal (v2) |
| 13 | `is_sports` | Category keyword matching (v4) |
| 14-29 | `q_length`, `q_word_count`, `q_avg_word_len`, `q_word_diversity`, `q_has_number`, `q_has_year`, `q_has_percent`, `q_has_dollar`, `q_has_date`, `q_starts_will`, `q_has_by`, `q_has_before`, `q_has_above`, `q_sentiment_pos`, `q_sentiment_neg`, `q_certainty` | NLP from question text (v5) |

### Ensemble

The Python sidecar serves a stacking ensemble: XGBoost + LightGBM + HistGBM + ExtraTrees + RandomForest with a meta-learner. The Rust bot can also run standalone with a pure-Rust XGBoost implementation (JSON tree traversal, no native dependencies).

### Prediction Tracking

Every prediction is logged to `prediction_log` with market price, model posterior, confidence, and edge. When markets resolve, Brier scores are computed per source:

- **Model Brier** vs **Market Brier** shows whether the model adds value over market prices
- **Skill metric**: `1 - (model_brier / market_brier)` — positive means the model outperforms

## CI/CD

GitHub Actions with GHCR-based deployment:

- **CI** (on push/PR): `cargo fmt --check` → `cargo clippy` → `cargo test`
- **Deploy** (on main push): builds both bot images → deploys to Hetzner VPS via SSH with changelog
- **Release** (manual `workflow_dispatch`): bumps version in both Cargo.toml files, tags, creates GitHub release

## Architecture Decision Records

| # | Title | Status | Date |
|---|-------|--------|------|
| [001](adr/001-baseline.md) | Baseline model audit | active | 2026-03-15 |
| [002](adr/002-fix-training-inconsistencies.md) | Fix training/live inconsistencies | complete | 2026-03-15 |
| [003](adr/003-fresh-data-fetch.md) | Fresh data fetch with deployment-window filter | complete | 2026-03-15 |
| [004](adr/004-online-learning.md) | Online learning: feature store + warm-start retraining | draft | 2026-03-15 |
| [005](adr/005-feature-improvements.md) | Temporal features, target encoding, SHAP analysis | active | 2026-03-15 |
| [006](adr/006-remove-dead-features.md) | Remove dead features (15 → 12) | active | 2026-03-16 |
| [007](adr/007-portfolio-correlation-check.md) | Portfolio correlation check via LLM | draft | 2026-03-17 |
| [008](adr/008-workspace-split.md) | Cargo workspace split (3 crates) | accepted | 2026-03-23 |
| [009](adr/009-profitability-overhaul.md) | Profitability overhaul: signal filters + recalibration | active | 2026-04-06 |
| [010](adr/010-remove-dataset-scripts.md) | Remove Jon-Becker dataset scripts | active | 2026-04-09 |
