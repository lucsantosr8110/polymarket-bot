from typing import Annotated

import asyncpg
from fastapi import APIRouter, Depends

from dashboard_api.db import get_pool
from dashboard_api.models import OverviewResponse

router = APIRouter(prefix="/api", tags=["overview"])


@router.get("/overview", response_model=OverviewResponse)
async def overview(pool: Annotated[asyncpg.Pool | None, Depends(get_pool)]) -> OverviewResponse:
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

    return OverviewResponse(**dict(row))


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
