import os
import time
from contextlib import asynccontextmanager

from fastapi import FastAPI

from dashboard_api.db import close_http_client, close_pool, create_http_client, create_pool
from dashboard_api.routes import bets, config_global, health, latency_metrics, overview, signals, strategies
from dashboard_api.websocket_logs import router as logs_router


@asynccontextmanager
async def lifespan(app: FastAPI):
    app.state.started_at = time.time()
    app.state.pool = await create_pool()
    app.state.http_client = create_http_client()
    app.state.bot_running = os.getenv("BOT_RUNNING", "true").lower() == "true"
    yield
    await close_pool(app.state.pool)
    await close_http_client(app.state.http_client)


app = FastAPI(title="Polymarket Dashboard API", version="0.1.0", lifespan=lifespan)

app.include_router(health.router)
app.include_router(overview.router)
app.include_router(bets.router)
app.include_router(signals.router)
app.include_router(strategies.router)
app.include_router(config_global.router)
app.include_router(latency_metrics.router)
app.include_router(logs_router)
