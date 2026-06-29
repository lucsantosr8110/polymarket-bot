from datetime import datetime
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field


class HealthResponse(BaseModel):
    status: Literal["ok"]
    db_connected: bool
    model_loaded: bool
    bot_running: bool
    uptime_seconds: int


class OverviewResponse(BaseModel):
    total_bankroll: float
    pnl_today: float
    pnl_week: float
    open_bets: int
    total_bets: int
    win_rate: float
    profit_factor: float
    signals_today: int
    last_scan: datetime | None


class OpenBetResponse(BaseModel):
    id: int
    market_id: str
    question: str
    side: str
    entry_price: float
    current_price: float
    shares: float
    cost: float
    pnl_unrealized: float
    placed_at: datetime
    category: str | None = None
    fee_paid: float | None = None
    fee_rate: float | None = None


class BetHistoryResponse(BaseModel):
    id: int
    market_id: str
    question: str
    side: str
    entry_price: float
    shares: float
    cost: float
    pnl: float | None
    won: bool | None
    placed_at: datetime
    resolved_at: datetime | None
    category: str | None = None
    fee_paid: float | None = None
    fee_rate: float | None = None


class RecentSignalResponse(BaseModel):
    status: Literal["accepted", "rejected"]
    market_id: str
    question: str
    reason: str | None = None
    side: str | None = None
    entry_price: float | None = None
    current_price: float | None = None
    estimated_prob: float | None = None
    edge: float | None = None
    confidence: float | None = None
    created_at: datetime


class StrategyPatch(BaseModel):
    model_config = ConfigDict(extra="forbid")

    min_edge: float | None = Field(default=None, ge=0)
    min_confidence: float | None = Field(default=None, ge=0, le=1)
    kelly_fraction: float | None = Field(default=None, ge=0, le=1.0)
    max_signals_per_day: int | None = Field(default=None, ge=0)
    min_bet: float | None = Field(default=None, ge=0)


RiskProfile = Literal["conservative", "balanced", "aggressive", "custom"]


class GlobalConfigPatch(BaseModel):
    # Live-reloadable fields consumed by the Rust bot's RuntimeGlobals (typed + bounded).
    scan_interval_mins: int | None = Field(default=None, ge=1)
    bet_scan_interval_mins: int | None = Field(default=None, ge=1)
    heartbeat_interval_mins: int | None = Field(default=None, ge=0)
    config_poll_interval_secs: int | None = Field(default=None, ge=1)
    slippage_pct: float | None = Field(default=None, ge=0, le=1)
    stop_loss_pct: float | None = Field(default=None, ge=0)
    exit_days_before_expiry: int | None = Field(default=None, ge=0)
    min_kelly_size: float | None = Field(default=None, ge=0, le=1)
    min_bet_price: float | None = Field(default=None, ge=0, le=1)
    max_ws_bets_per_day: int | None = Field(default=None, ge=0)
    alert_throttle_mins: int | None = Field(default=None, ge=0)
    ws_bet_cooldown_secs: int | None = Field(default=None, ge=0)
    price_alert_cooldown_secs: int | None = Field(default=None, ge=0)

    # Shared enum across TS/Pydantic/Rust; "custom" is injected by the dashboard.
    risk_profile: RiskProfile | None = None

    # Dashboard-compat / restart-required fields are still accepted but ignored by
    # live polling. extra="allow" preserved to avoid breaking the existing frontend
    # payload which sends the full config object on save.
    model_config = ConfigDict(extra="allow")


class LogMessage(BaseModel):
    timestamp: datetime
    level: str
    target: str
    message: str
    fields: dict[str, Any] = Field(default_factory=dict)
