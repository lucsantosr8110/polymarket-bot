import os
from datetime import datetime, timezone
from typing import Annotated

import asyncpg
import httpx
from fastapi import APIRouter, Depends

from dashboard_api.db import get_http_client, get_pool
from dashboard_api.models import OverviewResponse

router = APIRouter(prefix="/api", tags=["overview"])

PROMETHEUS_URL = os.getenv("PROMETHEUS_URL", "http://localhost:9090")


async def _last_bet_scan_at(client: httpx.AsyncClient) -> datetime | None:
    """Wall-clock time of the last completed bet_scan cycle, signal or not.

    Sourced from Prometheus (bot_operation_duration_seconds_count{operation=
    "bet_scan"}, incremented every cycle) rather than rejected_signals.MAX
    (created_at) — that SQL fallback only moves when a candidate market is
    found, so on a quiet cycle (signals_today=0, common with a tight
    bet_scan_interval_mins) it looks stuck even though the bot is scanning
    on schedule.
    """
    query = 'timestamp(bot_operation_duration_seconds_count{operation="bet_scan",step="full_cycle"})'
    try:
        resp = await client.get(f"{PROMETHEUS_URL}/api/v1/query", params={"query": query})
        resp.raise_for_status()
        result = resp.json().get("data", {}).get("result")
        if not result:
            return None
        return datetime.fromtimestamp(float(result[0]["value"][1]), tz=timezone.utc)
    except Exception:
        return None


@router.get("/overview", response_model=OverviewResponse)
async def overview(
    pool: Annotated[asyncpg.Pool | None, Depends(get_pool)],
    client: Annotated[httpx.AsyncClient, Depends(get_http_client)],
) -> OverviewResponse:
    if pool is None:
        return _empty_overview()

    row = await pool.fetchrow(
        """
        SELECT
            COALESCE((SELECT SUM(value_f64) FROM portfolio WHERE key LIKE 'bankroll:%'), 0.0) AS total_bankroll,
            COALESCE((SELECT SUM(pnl) FROM bets WHERE resolved_at > NOW() - INTERVAL '1 day'), 0.0) AS pnl_today,
            COALESCE((SELECT SUM(pnl) FROM bets WHERE resolved_at > NOW() - INTERVAL '7 days'), 0.0) AS pnl_week,
            COALESCE((SELECT COUNT(*) FROM bets WHERE resolved = false), 0) AS open_bets,
            COALESCE((SELECT COUNT(*) FROM bets), 0) AS total_bets,
            COALESCE((SELECT AVG(CASE WHEN won = true THEN 1.0 ELSE 0.0 END) FROM bets WHERE resolved = true), 0.0) AS win_rate,
            COALESCE((
                SELECT
                    CASE
                        WHEN ABS(SUM(CASE WHEN pnl < 0 THEN pnl ELSE 0 END)) = 0 THEN 0.0
                        ELSE SUM(CASE WHEN pnl > 0 THEN pnl ELSE 0 END) / ABS(SUM(CASE WHEN pnl < 0 THEN pnl ELSE 0 END))
                    END
                FROM bets
                WHERE resolved = true
            ), 0.0) AS profit_factor,
            COALESCE((SELECT COUNT(*) FROM rejected_signals WHERE created_at > NOW() - INTERVAL '1 day'), 0) AS signals_today,
            (SELECT MAX(created_at) FROM rejected_signals) AS last_scan
        """
    )

    data = dict(row)
    last_bet_scan_at = await _last_bet_scan_at(client)
    if last_bet_scan_at is not None:
        data["last_scan"] = last_bet_scan_at

    return OverviewResponse(**data)


def _empty_overview() -> OverviewResponse:
    return OverviewResponse(
        total_bankroll=0.0,
        pnl_today=0.0,
        pnl_week=0.0,
        open_bets=0,
        total_bets=0,
        win_rate=0.0,
        profit_factor=0.0,
        signals_today=0,
        last_scan=None,
    )
