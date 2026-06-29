from typing import Annotated, Any

import asyncpg
from fastapi import APIRouter, Depends, HTTPException

from dashboard_api.config_loader import get_runtime_config, merge_strategy, set_runtime_config
from dashboard_api.db import get_pool
from dashboard_api.models import StrategyPatch

router = APIRouter(prefix="/api/strategies", tags=["strategies"])


@router.get("")
async def get_strategies(pool: Annotated[asyncpg.Pool | None, Depends(get_pool)]) -> list[dict[str, Any]]:
    strategies = await get_runtime_config(pool, "strategies")
    if not isinstance(strategies, list):
        raise HTTPException(status_code=500, detail="strategies data corrupted: expected list")
    return strategies


@router.put("/{name}")
async def update_strategy(
    name: str,
    patch: StrategyPatch,
    pool: Annotated[asyncpg.Pool | None, Depends(get_pool)],
) -> dict[str, Any]:
    strategies = await get_runtime_config(pool, "strategies")
    if not isinstance(strategies, list):
        raise HTTPException(status_code=500, detail="strategies data corrupted: expected list")
    patch_data = patch.model_dump(exclude_none=True)
    try:
        updated = merge_strategy(strategies, name, patch_data)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=f"Strategy '{name}' not found") from exc

    await set_runtime_config(pool, "strategies", updated)
    return next(item for item in updated if item["name"].lower() == name.lower())
