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
    kelly_fraction: float | None = Field(default=None, ge=0)
    max_signals_per_day: int | None = Field(default=None, ge=0)
    min_bet: float | None = Field(default=None, ge=0)


class GlobalConfigPatch(BaseModel):
    model_config = ConfigDict(extra="allow")


class LogMessage(BaseModel):
    timestamp: datetime
    level: str
    target: str
    message: str
    fields: dict[str, Any] = Field(default_factory=dict)
