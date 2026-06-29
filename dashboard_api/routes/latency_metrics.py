import asyncio
import os
from typing import Annotated

import httpx
from fastapi import APIRouter, Depends

from dashboard_api.db import get_http_client
from dashboard_api.models import LatencyMetricsResponse

router = APIRouter(prefix="/api/metrics", tags=["metrics"])

# Matches the compose service name in production; override for local dev
# where Prometheus isn't on the same Docker network.
PROMETHEUS_URL = os.getenv("PROMETHEUS_URL", "http://localhost:9090")

OPERATIONS = [
    "bet_scan",
    "fetch_markets",
    "predict_batch",
    "place_bet",
    "housekeeping",
    "runtime_config_poll",
]


def _avg_query(operation: str) -> str:
    # bot_operation_duration_seconds is a classic bucketed histogram (fixed
    # buckets registered in common::metrics::init), so the average is
    # sum / count — histogram_avg() only exists for native histograms.
    #
    # Deliberately no rate([5m]): bet_scan/housekeeping/runtime_config_poll
    # run once every 1-10+ minutes, so a 5m window usually contains 0-1
    # samples. rate() over that few points is extrapolated from sparse data
    # and was observed to return +Inf/NaN/0 inconsistently. Cumulative
    # sum/count (i.e. the all-time average since the bot's last restart) is
    # the stable, correct metric for this sample frequency.
    return (
        f'sum(bot_operation_duration_seconds_sum{{operation="{operation}"}}) '
        f'/ sum(bot_operation_duration_seconds_count{{operation="{operation}"}})'
    )


def _p95_query(operation: str) -> str:
    # Same reasoning as _avg_query: quantile over raw cumulative bucket
    # counts (since last restart), not a rate()'d 5m window.
    return f'histogram_quantile(0.95, sum(bot_operation_duration_seconds_bucket{{operation="{operation}"}}) by (le))'


async def _query_value(client: httpx.AsyncClient, query: str) -> float | None:
    try:
        resp = await client.get(f"{PROMETHEUS_URL}/api/v1/query", params={"query": query})
        resp.raise_for_status()
        result = resp.json().get("data", {}).get("result")
        if not result:
            return None
        value = float(result[0]["value"][1])
        return value if value == value else None  # filters NaN (Prometheus "no data")
    except Exception:
        return None


@router.get("/latency", response_model=LatencyMetricsResponse)
async def get_latency(
    client: Annotated[httpx.AsyncClient, Depends(get_http_client)],
) -> LatencyMetricsResponse:
    """Avg/p95 latency per bot operation. Each field is independently null
    if Prometheus has no data for it yet or is unreachable."""
    field_queries = {
        f"{op}_{suffix}": query(op)
        for op in OPERATIONS
        for suffix, query in (("avg", _avg_query), ("p95", _p95_query))
    }

    fields = list(field_queries.keys())
    values = await asyncio.gather(*(_query_value(client, field_queries[f]) for f in fields))
    return LatencyMetricsResponse(**dict(zip(fields, values)))
