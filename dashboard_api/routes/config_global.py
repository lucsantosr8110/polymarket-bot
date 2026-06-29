from typing import Annotated, Any

import asyncpg
from fastapi import APIRouter, Depends

from dashboard_api.config_loader import get_runtime_config, merge_global_config, set_runtime_config
from dashboard_api.db import get_pool
from dashboard_api.models import GlobalConfigPatch

router = APIRouter(prefix="/api/config", tags=["config"])


@router.get("/global")
async def get_global_config(pool: Annotated[asyncpg.Pool | None, Depends(get_pool)]) -> dict[str, Any]:
    config = await get_runtime_config(pool, "global")
    return dict(config)


@router.put("/global")
async def update_global_config(
    patch: GlobalConfigPatch,
    pool: Annotated[asyncpg.Pool | None, Depends(get_pool)],
) -> dict[str, Any]:
    current = dict(await get_runtime_config(pool, "global"))
    updated = merge_global_config(current, patch.model_dump(exclude_none=True))
    await set_runtime_config(pool, "global", updated)
    return updated
