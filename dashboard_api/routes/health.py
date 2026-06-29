import os
import time
from typing import Annotated

import asyncpg
from fastapi import APIRouter, Depends, Request

from dashboard_api.db import get_pool
from dashboard_api.models import HealthResponse

router = APIRouter(prefix="/api", tags=["health"])


async def _db_connected(pool: asyncpg.Pool | None) -> bool:
    if pool is None:
        return False
    try:
        await pool.fetchval("SELECT 1")
        return True
    except Exception:
        return False


async def _model_loaded(pool: asyncpg.Pool | None) -> bool:
    if pool is None:
        return False
    try:
        return bool(
            await pool.fetchval(
                """
                SELECT EXISTS (
                    SELECT 1 FROM prediction_log
                    WHERE created_at > NOW() - INTERVAL '7 days'
                )
                """
            )
        )
    except Exception:
        return False


@router.get("/health", response_model=HealthResponse)
async def health(
    request: Request,
    pool: Annotated[asyncpg.Pool | None, Depends(get_pool)],
) -> HealthResponse:
    return HealthResponse(
        status="ok",
        db_connected=await _db_connected(pool),
        model_loaded=await _model_loaded(pool),
        bot_running=bool(getattr(request.app.state, "bot_running", True)),
        uptime_seconds=int(time.time() - request.app.state.started_at),
    )
