import asyncio
import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from fastapi import APIRouter, WebSocket, WebSocketDisconnect

router = APIRouter(tags=["logs"])


@router.websocket("/api/logs/stream")
async def stream_logs(websocket: WebSocket) -> None:
    await websocket.accept()
    log_path = Path(os.getenv("BOT_LOG_FILE", "logs/bot.log"))
    try:
        if not log_path.exists():
            await websocket.send_json(_log_message("INFO", "dashboard_api.logs", "Log file not found; waiting for bot logs", {}))
        await _tail_file(websocket, log_path)
    except WebSocketDisconnect:
        return


async def _tail_file(websocket: WebSocket, log_path: Path) -> None:
    position = log_path.stat().st_size if log_path.exists() else 0
    was_present = log_path.exists()
    while True:
        if not log_path.exists():
            if was_present:
                await websocket.send_json(
                    _log_message("WARN", "dashboard_api.logs", "Log file disappeared; waiting for it to reappear", {})
                )
                was_present = False
                position = 0
            await asyncio.sleep(1)
            continue

        was_present = True
        with log_path.open("r", encoding="utf-8", errors="replace") as handle:
            handle.seek(position)
            while line := handle.readline():
                position = handle.tell()
                await websocket.send_json(_parse_log_line(line.strip()))
        await asyncio.sleep(1)


def _parse_log_line(line: str) -> dict[str, Any]:
    if not line:
        return _log_message("INFO", "dashboard_api.logs", "", {})

    try:
        payload = json.loads(line)
    except json.JSONDecodeError:
        return _log_message("INFO", "bot", line, {})

    timestamp = payload.get("timestamp") or payload.get("ts") or _now()
    level = str(payload.get("level", "INFO")).upper()
    target = str(payload.get("target") or payload.get("logger") or "bot")
    message = str(payload.get("message") or payload.get("msg") or "")
    fields = {key: value for key, value in payload.items() if key not in {"timestamp", "ts", "level", "target", "logger", "message", "msg"}}
    return _log_message(level, target, message, fields, timestamp=timestamp)


def _log_message(
    level: str,
    target: str,
    message: str,
    fields: dict[str, Any],
    *,
    timestamp: str | None = None,
) -> dict[str, Any]:
    return {
        "timestamp": timestamp or _now(),
        "level": level,
        "target": target,
        "message": message,
        "fields": fields,
    }


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()
