use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use tokio::sync::RwLock;

use crate::config::AppConfig;
use crate::strategy::StrategyProfile;

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeStrategyConfig {
    pub name: String,
    #[serde(default)]
    pub min_edge: f64,
    #[serde(default)]
    pub min_confidence: f64,
    #[serde(default)]
    pub kelly_fraction: f64,
    #[serde(default)]
    pub max_signals_per_day: usize,
    #[serde(default)]
    pub min_bet: f64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GlobalRuntimeConfig {
    pub scan_interval_mins: Option<u64>,
    pub news_scan_interval_mins: Option<u64>,
    pub bet_scan_interval_mins: Option<u64>,
    pub heartbeat_interval_mins: Option<u64>,
    pub config_poll_interval_secs: Option<u64>,
    pub active_strategies: Option<Vec<String>>,
    pub risk_profile: Option<String>,
    pub news_enabled: Option<bool>,
    pub slippage_pct: Option<f64>,
    pub min_volume: Option<f64>,
    pub min_book_depth: Option<f64>,
    pub kelly_fraction: Option<f64>,
    pub max_days_to_expiry: Option<i64>,
    pub max_llm_candidates: Option<usize>,
    pub max_model_candidates: Option<usize>,
    pub min_effective_edge: Option<f64>,
    pub llm_model: Option<String>,
    pub llm_models: Option<Vec<String>>,
    pub retrain_interval_hours: Option<u64>,
    pub consensus_agents: Option<usize>,
    pub calibration_min_samples: Option<usize>,
    pub max_markets_fetch: Option<usize>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub strategy_bankroll: Option<f64>,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub exit_days_before_expiry: Option<i64>,
    pub block_sports: Option<bool>,
    pub block_yes_side: Option<bool>,
    pub lr_damping: Option<f64>,
    pub min_kelly_size: Option<f64>,
    pub min_bet_price: Option<f64>,
    pub model_sidecar_url: Option<String>,
    pub sidecar_timeout_secs: Option<u64>,
    pub sidecar_retries: Option<usize>,
    pub sidecar_retry_delay_secs: Option<u64>,
    pub http_timeout_secs: Option<u64>,
    pub news_fetch_timeout_secs: Option<u64>,
    pub alert_throttle_mins: Option<u64>,
    pub ws_bet_cooldown_secs: Option<u64>,
    pub price_alert_cooldown_secs: Option<u64>,
    pub max_ws_bets_per_day: Option<usize>,
    pub ws_reconnect_delay_secs: Option<u64>,
    pub ws_min_price_delta: Option<f64>,
    pub ws_min_trade_usd: Option<f64>,
    pub metrics_port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct RuntimeGlobals {
    pub scan_interval_mins: u64,
    pub bet_scan_interval_mins: u64,
    pub heartbeat_interval_mins: u64,
    pub config_poll_interval_secs: u64,
    pub slippage_pct: f64,
    pub stop_loss_pct: f64,
    pub exit_days_before_expiry: i64,
    pub min_kelly_size: f64,
    pub min_bet_price: f64,
    pub max_ws_bets_per_day: usize,
    pub alert_throttle_mins: u64,
    pub ws_bet_cooldown_secs: u64,
    pub price_alert_cooldown_secs: u64,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfigSnapshot {
    pub strategies: Vec<StrategyProfile>,
    pub global: RuntimeGlobals,
    pub updated_at: Option<DateTime<Utc>>,
    pub fingerprint: u64,
}

impl RuntimeConfigSnapshot {
    pub fn from_app_config(cfg: &AppConfig) -> Self {
        Self {
            strategies: StrategyProfile::from_config(cfg),
            global: RuntimeGlobals::from_app_config(cfg),
            updated_at: None,
            fingerprint: 0,
        }
    }
}

impl RuntimeGlobals {
    pub fn from_app_config(cfg: &AppConfig) -> Self {
        Self {
            scan_interval_mins: cfg.scan_interval_mins,
            bet_scan_interval_mins: cfg.bet_scan_interval_mins,
            heartbeat_interval_mins: cfg.heartbeat_interval_mins,
            config_poll_interval_secs: cfg.config_poll_interval_secs,
            slippage_pct: cfg.slippage_pct,
            stop_loss_pct: cfg.stop_loss_pct,
            exit_days_before_expiry: cfg.exit_days_before_expiry,
            min_kelly_size: cfg.min_kelly_size,
            min_bet_price: cfg.min_bet_price,
            max_ws_bets_per_day: cfg.max_ws_bets_per_day,
            alert_throttle_mins: cfg.alert_throttle_mins,
            ws_bet_cooldown_secs: cfg.ws_bet_cooldown_secs,
            price_alert_cooldown_secs: cfg.price_alert_cooldown_secs,
        }
    }

    fn overlay(cfg: &AppConfig, runtime: &GlobalRuntimeConfig) -> Self {
        Self {
            scan_interval_mins: runtime.scan_interval_mins.unwrap_or(cfg.scan_interval_mins),
            bet_scan_interval_mins: runtime
                .bet_scan_interval_mins
                .unwrap_or(cfg.bet_scan_interval_mins),
            heartbeat_interval_mins: runtime
                .heartbeat_interval_mins
                .unwrap_or(cfg.heartbeat_interval_mins),
            config_poll_interval_secs: runtime
                .config_poll_interval_secs
                .unwrap_or(cfg.config_poll_interval_secs),
            slippage_pct: runtime.slippage_pct.unwrap_or(cfg.slippage_pct),
            stop_loss_pct: runtime.stop_loss_pct.unwrap_or(cfg.stop_loss_pct),
            exit_days_before_expiry: runtime
                .exit_days_before_expiry
                .unwrap_or(cfg.exit_days_before_expiry),
            min_kelly_size: runtime.min_kelly_size.unwrap_or(cfg.min_kelly_size),
            min_bet_price: runtime.min_bet_price.unwrap_or(cfg.min_bet_price),
            max_ws_bets_per_day: runtime
                .max_ws_bets_per_day
                .unwrap_or(cfg.max_ws_bets_per_day),
            alert_throttle_mins: runtime
                .alert_throttle_mins
                .unwrap_or(cfg.alert_throttle_mins),
            ws_bet_cooldown_secs: runtime
                .ws_bet_cooldown_secs
                .unwrap_or(cfg.ws_bet_cooldown_secs),
            price_alert_cooldown_secs: runtime
                .price_alert_cooldown_secs
                .unwrap_or(cfg.price_alert_cooldown_secs),
        }
    }
}

impl RuntimeStrategyConfig {
    fn into_profile(self) -> StrategyProfile {
        StrategyProfile {
            name: self.name.to_lowercase(),
            kelly_fraction: self.kelly_fraction,
            min_effective_edge: self.min_edge,
            min_confidence: self.min_confidence,
            max_signals_per_day: self.max_signals_per_day,
            min_bet: self.min_bet,
        }
    }
}

pub async fn fetch_runtime_config_snapshot(
    pool: &PgPool,
    cfg: &AppConfig,
) -> Result<RuntimeConfigSnapshot> {
    let rows = sqlx::query(
        "SELECT key, value, updated_at FROM runtime_config WHERE key IN ('strategies', 'global')",
    )
    .fetch_all(pool)
    .await
    .context("failed to query runtime_config")?;

    let mut strategies: Option<Vec<StrategyProfile>> = None;
    let mut global = GlobalRuntimeConfig::default();
    let mut updated_at: Option<DateTime<Utc>> = None;
    let mut fingerprint_inputs: Vec<(String, serde_json::Value)> = Vec::new();

    for row in rows {
        let key: String = row.try_get("key")?;
        let value: serde_json::Value = row.try_get("value")?;
        let row_updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        updated_at = Some(updated_at.map_or(row_updated_at, |current| current.max(row_updated_at)));
        fingerprint_inputs.push((key.clone(), value.clone()));

        match key.as_str() {
            "strategies" => {
                let parsed: Vec<RuntimeStrategyConfig> =
                    serde_json::from_value(value).context("invalid runtime strategies JSON")?;
                let profiles: Vec<StrategyProfile> = parsed
                    .into_iter()
                    .map(RuntimeStrategyConfig::into_profile)
                    .collect();
                if !profiles.is_empty() {
                    strategies = Some(profiles);
                }
            }
            "global" => {
                global = serde_json::from_value(value).context("invalid runtime global JSON")?;
            }
            _ => {}
        }
    }

    let profiles = filter_active_strategies(
        strategies.unwrap_or_else(|| StrategyProfile::from_config(cfg)),
        global.active_strategies.as_deref(),
        cfg,
    );

    Ok(RuntimeConfigSnapshot {
        strategies: profiles,
        global: RuntimeGlobals::overlay(cfg, &global),
        updated_at,
        fingerprint: runtime_fingerprint(&fingerprint_inputs),
    })
}

fn runtime_fingerprint(values: &[(String, serde_json::Value)]) -> u64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (key, value) in ordered {
        key.hash(&mut hasher);
        value.to_string().hash(&mut hasher);
    }
    hasher.finish()
}

fn filter_active_strategies(
    profiles: Vec<StrategyProfile>,
    active: Option<&[String]>,
    cfg: &AppConfig,
) -> Vec<StrategyProfile> {
    let Some(active) = active else {
        return profiles;
    };
    if active.is_empty() {
        return profiles;
    }

    let active_names: HashSet<String> = active.iter().map(|name| name.to_lowercase()).collect();
    let filtered: Vec<StrategyProfile> = profiles
        .into_iter()
        .filter(|profile| active_names.contains(&profile.name.to_lowercase()))
        .collect();

    if filtered.is_empty() {
        tracing::warn!(
            "runtime_config active_strategies matched no profiles; using AppConfig fallback"
        );
        StrategyProfile::from_config(cfg)
    } else {
        filtered
    }
}

pub async fn reload_runtime_config(
    pool: &PgPool,
    cfg: &AppConfig,
    store: &RwLock<RuntimeConfigSnapshot>,
) -> Result<bool> {
    let next = fetch_runtime_config_snapshot(pool, cfg).await?;
    let current_fingerprint = store.read().await.fingerprint;
    if next.fingerprint == current_fingerprint {
        return Ok(false);
    }

    let strategy_count = next.strategies.len();
    let updated_at = next.updated_at;
    let fingerprint = next.fingerprint;
    *store.write().await = next;
    tracing::info!(
        ?updated_at,
        fingerprint,
        strategies = strategy_count,
        "Runtime config reloaded from database"
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_strategy_runtime_config() {
        let json = serde_json::json!({
            "name": "Balanced",
            "min_edge": 0.07,
            "min_confidence": 0.45,
            "kelly_fraction": 0.20,
            "max_signals_per_day": 4,
            "min_bet": 7.5
        });
        let parsed: RuntimeStrategyConfig = serde_json::from_value(json).unwrap();
        let profile = parsed.into_profile();

        assert_eq!(profile.name, "balanced");
        assert_eq!(profile.min_effective_edge, 0.07);
        assert_eq!(profile.min_confidence, 0.45);
        assert_eq!(profile.kelly_fraction, 0.20);
        assert_eq!(profile.max_signals_per_day, 4);
        assert_eq!(profile.min_bet, 7.5);
    }

    #[test]
    fn strategy_with_missing_field_uses_default() {
        // A single missing field must not break the whole strategy array reload.
        let json = serde_json::json!({
            "name": "Balanced",
            "min_edge": 0.07,
            "kelly_fraction": 0.20,
            "max_signals_per_day": 4,
            "min_bet": 7.5
        });
        let parsed: RuntimeStrategyConfig = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.min_confidence, 0.0);
    }

    #[test]
    fn global_overlay_uses_runtime_values_with_config_fallback() {
        let cfg = AppConfig::test_default();
        let runtime = GlobalRuntimeConfig {
            bet_scan_interval_mins: Some(3),
            stop_loss_pct: Some(0.25),
            min_kelly_size: Some(0.04),
            min_bet_price: Some(0.2),
            max_ws_bets_per_day: Some(8),
            ..GlobalRuntimeConfig::default()
        };

        let globals = RuntimeGlobals::overlay(&cfg, &runtime);

        assert_eq!(globals.bet_scan_interval_mins, 3);
        assert_eq!(globals.scan_interval_mins, cfg.scan_interval_mins);
        assert_eq!(globals.stop_loss_pct, 0.25);
        assert_eq!(globals.min_kelly_size, 0.04);
        assert_eq!(globals.min_bet_price, 0.2);
        assert_eq!(globals.max_ws_bets_per_day, 8);
    }

    #[test]
    fn active_strategies_filter_profiles() {
        let cfg = AppConfig::test_default();
        let profiles = StrategyProfile::from_config(&cfg);
        let active = vec!["balanced".to_string()];

        let filtered = filter_active_strategies(profiles, Some(&active), &cfg);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "balanced");
    }

    #[test]
    fn fingerprint_changes_when_json_changes_even_with_same_keys() {
        let left = vec![
            (
                "global".to_string(),
                serde_json::json!({"stop_loss_pct": 0.5}),
            ),
            ("strategies".to_string(), serde_json::json!([])),
        ];
        let right = vec![
            ("strategies".to_string(), serde_json::json!([])),
            (
                "global".to_string(),
                serde_json::json!({"stop_loss_pct": 0.25}),
            ),
        ];

        assert_ne!(runtime_fingerprint(&left), runtime_fingerprint(&right));
    }

    #[test]
    fn fingerprint_is_order_independent_by_key() {
        let left = vec![
            (
                "global".to_string(),
                serde_json::json!({"stop_loss_pct": 0.5}),
            ),
            (
                "strategies".to_string(),
                serde_json::json!([{"name":"Balanced"}]),
            ),
        ];
        let right = vec![
            (
                "strategies".to_string(),
                serde_json::json!([{"name":"Balanced"}]),
            ),
            (
                "global".to_string(),
                serde_json::json!({"stop_loss_pct": 0.5}),
            ),
        ];

        assert_eq!(runtime_fingerprint(&left), runtime_fingerprint(&right));
    }
}
