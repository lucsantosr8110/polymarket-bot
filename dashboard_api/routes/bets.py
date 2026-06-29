import asyncio
import json
from datetime import date
from typing import Annotated

import asyncpg
import httpx
from fastapi import APIRouter, Depends, Query

from dashboard_api.db import get_http_client, get_pool
from dashboard_api.models import BetHistoryResponse, OpenBetResponse

router = APIRouter(prefix="/api/bets", tags=["bets"])

# Literal, not env-configurable: this URL never changes and threading an
# override through every dashboard route isn't worth the blast radius —
# mirrors the same call in trading-bot's open_bets_summary_filtered.
GAMMA_API = "https://gamma-api.polymarket.com"


async def _fetch_yes_price(client: httpx.AsyncClient, market_id: str) -> float | None:
    try:
        resp = await client.get(f"{GAMMA_API}/markets/{market_id}")
        resp.raise_for_status()
        outcome_prices = resp.json().get("outcomePrices")
        if not outcome_prices:
            return None
        prices = json.loads(outcome_prices) if isinstance(outcome_prices, str) else outcome_prices
        return float(prices[0])
    except Exception:
        return None


@router.get("/open", response_model=list[OpenBetResponse])
async def open_bets(
    pool: Annotated[asyncpg.Pool | None, Depends(get_pool)],
    client: Annotated[httpx.AsyncClient, Depends(get_http_client)],
) -> list[OpenBetResponse]:
    if pool is None:
        return []

    rows = await pool.fetch(
        """
        SELECT
            id, market_id, question, side, entry_price,
            shares, cost,
            placed_at, category, fee_paid, fee_rate
        FROM bets
        WHERE resolved = false
        ORDER BY placed_at DESC
        """
    )

    yes_prices = await asyncio.gather(*(_fetch_yes_price(client, row["market_id"]) for row in rows))

    bets = []
    for row, yes_price in zip(rows, yes_prices):
        data = dict(row)
        if yes_price is None:
            # Gamma unreachable / market not found — fall back to entry price
            # (flat P&L) rather than dropping the row.
            current_price = data["entry_price"]
        else:
            current_price = yes_price if data["side"] == "Yes" else 1.0 - yes_price
        data["current_price"] = current_price
        data["pnl_unrealized"] = data["shares"] * current_price - data["cost"]
        bets.append(OpenBetResponse(**data))

    return bets


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
