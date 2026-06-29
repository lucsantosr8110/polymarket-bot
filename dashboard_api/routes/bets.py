from datetime import date
from typing import Annotated

import asyncpg
from fastapi import APIRouter, Depends, Query

from dashboard_api.db import get_pool
from dashboard_api.models import BetHistoryResponse, OpenBetResponse

router = APIRouter(prefix="/api/bets", tags=["bets"])


@router.get("/open", response_model=list[OpenBetResponse])
async def open_bets(pool: Annotated[asyncpg.Pool | None, Depends(get_pool)]) -> list[OpenBetResponse]:
    if pool is None:
        return []

    rows = await pool.fetch(
        """
        SELECT
            id, market_id, question, side, entry_price,
            entry_price AS current_price,
            shares, cost,
            0.0 AS pnl_unrealized,
            placed_at, category, fee_paid, fee_rate
        FROM bets
        WHERE resolved = false
        ORDER BY placed_at DESC
        """
    )
    return [OpenBetResponse(**dict(row)) for row in rows]


@router.get("/history", response_model=list[BetHistoryResponse])
async def bet_history(
    pool: Annotated[asyncpg.Pool | None, Depends(get_pool)],
    limit: Annotated[int, Query(ge=1, le=500)] = 50,
    offset: Annotated[int, Query(ge=0)] = 0,
    from_date: Annotated[date | None, Query(alias="from")] = None,
    to_date: Annotated[date | None, Query(alias="to")] = None,
) -> list[BetHistoryResponse]:
    if pool is None:
        return []

    rows = await pool.fetch(
        """
        SELECT
            id, market_id, question, side, entry_price, shares, cost,
            pnl, won, placed_at, resolved_at, category, fee_paid, fee_rate
        FROM bets
        WHERE resolved = true
          AND ($1::date IS NULL OR resolved_at::date >= $1)
          AND ($2::date IS NULL OR resolved_at::date <= $2)
        ORDER BY resolved_at DESC NULLS LAST, placed_at DESC
        LIMIT $3 OFFSET $4
        """,
        from_date,
        to_date,
        limit,
        offset,
    )
    return [BetHistoryResponse(**dict(row)) for row in rows]
