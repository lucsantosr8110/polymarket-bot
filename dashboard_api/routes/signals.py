from typing import Annotated

import asyncpg
from fastapi import APIRouter, Depends, Query

from dashboard_api.db import get_pool
from dashboard_api.models import RecentSignalResponse

router = APIRouter(prefix="/api/signals", tags=["signals"])


@router.get("/recent", response_model=list[RecentSignalResponse])
async def recent_signals(
    pool: Annotated[asyncpg.Pool | None, Depends(get_pool)],
    limit: Annotated[int, Query(ge=1, le=200)] = 20,
) -> list[RecentSignalResponse]:
    if pool is None:
        return []

    rows = await pool.fetch(
        """
        SELECT *
        FROM (
            SELECT
                'accepted' AS status,
                market_id,
                question,
                NULL::text AS reason,
                side,
                entry_price,
                NULL::double precision AS current_price,
                estimated_prob,
                edge,
                confidence,
                placed_at AS created_at
            FROM bets
            UNION ALL
            SELECT
                'rejected' AS status,
                market_id,
                question,
                reason,
                NULL::text AS side,
                NULL::double precision AS entry_price,
                current_price,
                estimated_prob,
                edge,
                confidence,
                created_at
            FROM rejected_signals
        ) signals
        ORDER BY created_at DESC
        LIMIT $1
        """,
        limit,
    )
    return [RecentSignalResponse(**dict(row)) for row in rows]
