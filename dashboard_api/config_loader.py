from __future__ import annotations

from copy import deepcopy
import json
from typing import TYPE_CHECKING
from typing import Any

if TYPE_CHECKING:
    import asyncpg

RuntimeJson = dict[str, Any] | list[dict[str, Any]]

DEFAULT_STRATEGIES: list[dict[str, Any]] = [
    {
        "name": "Aggressive",
        "min_edge": 0.05,
        "min_confidence": 0.40,
        "kelly_fraction": 0.50,
        "max_signals_per_day": 10,
        "min_bet": 5.0,
    },
    {
        "name": "Balanced",
        "min_edge": 0.06,
        "min_confidence": 0.40,
        "kelly_fraction": 0.25,
        "max_signals_per_day": 5,
        "min_bet": 5.0,
    },
    {
        "name": "Conservative",
        "min_edge": 0.08,
        "min_confidence": 0.50,
        "kelly_fraction": 0.15,
        "max_signals_per_day": 3,
        "min_bet": 15.0,
    },
]

DEFAULT_GLOBAL_CONFIG: dict[str, Any] = {
    "scan_interval_mins": 30,
    "news_scan_interval_mins": 10,
    "bet_scan_interval_mins": 10,
    "heartbeat_interval_mins": 60,
    "retrain_interval_hours": 24,
    "model_sidecar_url": "",
    "llm_model": "gpt-4o",
    "llm_models": [
        "openai/gpt-oss-120b:free",
        "nvidia/nemotron-3-super-120b-a12b:free",
        "google/gemma-4-31b-it:free",
        "openai/gpt-oss-20b:free",
    ],
    "news_enabled": False,
    "slippage_pct": 0.01,
    "fee_pct_default": 0.0,
    "fee_pct_crypto": 0.018,
    "fee_pct_sports": 0.0075,
    "fee_pct_politics": 0.01,
    "fee_pct_finance": 0.01,
    "fee_pct_other": 0.0125,
    "min_volume": 1000.0,
    "min_book_depth": 200.0,
    "kelly_fraction": 0.25,
    "max_days_to_expiry": 14,
    "max_llm_candidates": 1,
    "max_model_candidates": 15,
    "min_effective_edge": 0.08,
    "consensus_agents": 2,
    "calibration_min_samples": 20,
    "max_markets_fetch": 1000,
    "min_price": 0.03,
    "max_price": 0.97,
    "strategy_bankroll": 300.0,
    "active_strategies": ["aggressive", "balanced", "conservative"],
    "stop_loss_pct": 999.0,
    "exit_days_before_expiry": 0,
    "block_sports": True,
    "block_yes_side": True,
    "lr_damping": 0.5,
    "min_kelly_size": 0.02,
    "min_bet_price": 0.15,
    "http_timeout_secs": 30,
    "news_fetch_timeout_secs": 15,
    "sidecar_timeout_secs": 10,
    "sidecar_retries": 3,
    "sidecar_retry_delay_secs": 2,
    "alert_throttle_mins": 15,
    "ws_bet_cooldown_secs": 600,
    "price_alert_cooldown_secs": 3600,
    "max_ws_bets_per_day": 3,
    "ws_reconnect_delay_secs": 5,
    "ws_min_price_delta": 0.03,
    "ws_min_trade_usd": 500.0,
    "metrics_port": 9000,
}

_MEMORY_CONFIG: dict[str, RuntimeJson] = {
    "strategies": deepcopy(DEFAULT_STRATEGIES),
    "global": deepcopy(DEFAULT_GLOBAL_CONFIG),
}


def _default_for_key(key: str) -> RuntimeJson:
    if key == "strategies":
        return deepcopy(DEFAULT_STRATEGIES)
    if key == "global":
        return deepcopy(DEFAULT_GLOBAL_CONFIG)
    raise KeyError(key)


async def get_runtime_config(pool: "asyncpg.Pool | None", key: str) -> RuntimeJson:
    if pool is None:
        return deepcopy(_MEMORY_CONFIG.get(key, _default_for_key(key)))

    row = await pool.fetchrow("SELECT value FROM runtime_config WHERE key = $1", key)
    if row is None:
        return _default_for_key(key)
    value = row["value"]
    if isinstance(value, str):
        return json.loads(value)
    return value


async def set_runtime_config(pool: "asyncpg.Pool | None", key: str, value: RuntimeJson) -> RuntimeJson:
    if pool is None:
        _MEMORY_CONFIG[key] = deepcopy(value)
        return deepcopy(value)

    await pool.execute(
        """
        INSERT INTO runtime_config (key, value, updated_at)
        VALUES ($1, $2::jsonb, NOW())
        ON CONFLICT (key)
        DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()
        """,
        key,
        json.dumps(value),
    )
    return value


def merge_strategy(
    strategies: list[dict[str, Any]],
    name: str,
    patch: dict[str, Any],
) -> list[dict[str, Any]]:
    found = False
    updated: list[dict[str, Any]] = []
    for strategy in strategies:
        if strategy.get("name", "").lower() == name.lower():
            found = True
            updated.append({**strategy, **patch, "name": strategy["name"]})
        else:
            updated.append({**strategy})

    if not found:
        raise KeyError(name)
    return updated


def merge_global_config(config: dict[str, Any], patch: dict[str, Any]) -> dict[str, Any]:
    return {**config, **patch}
