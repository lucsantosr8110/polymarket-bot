import os
import sys
from collections.abc import AsyncIterator

import asyncpg
import httpx
from dotenv import load_dotenv
from fastapi import Request


async def create_pool() -> asyncpg.Pool | None:
    load_dotenv()
    database_url = os.getenv("DATABASE_URL")
    if not database_url:
        return None
    try:
        return await asyncpg.create_pool(database_url, min_size=1, max_size=5, timeout=2)
    except Exception as exc:
        print(f"dashboard_api: database unavailable at startup: {exc}", file=sys.stderr)
        return None


async def close_pool(pool: asyncpg.Pool | None) -> None:
    if pool is not None:
        await pool.close()


async def get_pool(request: Request) -> AsyncIterator[asyncpg.Pool | None]:
    yield request.app.state.pool


def create_http_client() -> httpx.AsyncClient:
    return httpx.AsyncClient(timeout=5.0)


async def close_http_client(client: httpx.AsyncClient) -> None:
    await client.aclose()


async def get_http_client(request: Request) -> AsyncIterator[httpx.AsyncClient]:
    yield request.app.state.http_client
