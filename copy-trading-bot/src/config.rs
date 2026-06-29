use confique::Config;

#[derive(Debug, Config)]
pub struct CopyTradingConfig {
    /// Postgres connection string.
    #[config(env = "DATABASE_URL")]
    pub database_url: String,

    // --- Telegram ---
    #[config(env = "TELEGRAM_BOT_TOKEN")]
    pub telegram_bot_token: String,

    #[config(env = "TELEGRAM_CHAT_ID")]
    pub telegram_chat_id: String,

    // --- Copy trading ---
    /// Copy trading poll interval in minutes.
    #[config(env = "COPY_TRADE_INTERVAL_MINS", default = 1)]
    pub copy_trade_interval_mins: u64,

    // --- Betting ---
    /// Slippage assumption as a fraction (0.01 = 1%).
    #[config(env = "SLIPPAGE_PCT", default = 0.01)]
    pub slippage_pct: f64,

    /// Fee assumption as a fraction (0.02 = 2%).
    #[config(env = "FEE_PCT", default = 0.02)]
    pub fee_pct: f64,

    /// Port for the Prometheus metrics HTTP endpoint.
    #[config(env = "METRICS_PORT", default = 9001)]
    pub metrics_port: u16,

    // --- Copy-trading specific ---
    /// Polymarket data-API base URL (leaderboard, trader activity).
    #[config(env = "COPY_DATA_API_URL", default = "https://data-api.polymarket.com")]
    pub copy_data_api_url: String,

    /// Polymarket gamma-API base URL (market lookup, price, resolution).
    #[config(env = "COPY_GAMMA_API_URL", default = "https://gamma-api.polymarket.com")]
    pub copy_gamma_api_url: String,

    /// HTTP client timeout (seconds) for data-API/gamma-API requests.
    #[config(env = "COPY_REQUEST_TIMEOUT_SECS", default = 15)]
    pub copy_request_timeout_secs: u64,

    /// Number of traders shown per period section in the inline leaderboard reply.
    #[config(env = "COPY_LEADERBOARD_SECTION_LIMIT", default = 5)]
    pub copy_leaderboard_section_limit: usize,

    /// Trades older than this (seconds) are skipped — price has likely moved too far.
    #[config(env = "COPY_STALE_TRADE_SECS", default = 300)]
    pub copy_stale_trade_secs: i64,

    /// Default bankroll for a newly followed trader.
    #[config(env = "COPY_STARTING_BANKROLL", default = 1000.0)]
    pub copy_starting_bankroll: f64,

    /// Kelly fraction multiplier for copy-trade sizing (quarter-Kelly default).
    #[config(env = "COPY_KELLY_FRACTION", default = 0.25)]
    pub copy_kelly_fraction: f64,

    /// Minimum copy-trade bet size.
    #[config(env = "COPY_MIN_BET", default = 3.0)]
    pub copy_min_bet: f64,

    /// Maximum allowed price drift from the trader's entry before skipping.
    #[config(env = "COPY_MAX_PRICE_DRIFT", default = 0.05)]
    pub copy_max_price_drift: f64,

    /// Copy housekeeping loop interval in minutes (resolution checks).
    #[config(env = "COPY_HOUSEKEEPING_INTERVAL_MINS", default = 5)]
    pub copy_housekeeping_interval_mins: u64,

    /// Telegram command-polling interval in seconds.
    #[config(env = "COPY_TELEGRAM_POLL_SECS", default = 3)]
    pub copy_telegram_poll_secs: u64,

    /// Delay (seconds) between Postgres connection retry attempts on boot.
    #[config(env = "COPY_DB_RETRY_DELAY_SECS", default = 3)]
    pub copy_db_retry_delay_secs: u64,
}

impl CopyTradingConfig {
    pub fn load() -> Result<Self, confique::Error> {
        Self::builder().env().load()
    }
}
