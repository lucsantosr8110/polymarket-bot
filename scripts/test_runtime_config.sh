#!/usr/bin/env bash
set -euo pipefail

DB_URL="${DATABASE_URL:-postgres://bot:bot@localhost:5432/polymarket}"
API_URL="${DASHBOARD_API_URL:-http://localhost:8001}"
STRATEGY="${1:-Balanced}"
CONFIDENCE="${2:-0.30}"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

need psql
need curl

echo "Checking runtime_config table..."
psql "$DB_URL" -v ON_ERROR_STOP=1 -c "SELECT key, updated_at FROM runtime_config WHERE key IN ('strategies','global');"

echo "Updating strategy via dashboard API: $STRATEGY min_confidence=$CONFIDENCE"
curl -fsS -X PUT "$API_URL/api/strategies/$STRATEGY" \
  -H "Content-Type: application/json" \
  -d "{\"min_confidence\":$CONFIDENCE}" >/tmp/runtime_config_strategy.json
cat /tmp/runtime_config_strategy.json
echo

echo "Setting fast polling via dashboard API: config_poll_interval_secs=10"
curl -fsS -X PUT "$API_URL/api/config/global" \
  -H "Content-Type: application/json" \
  -d '{"config_poll_interval_secs":10}' >/tmp/runtime_config_global.json
cat /tmp/runtime_config_global.json
echo

echo "Database values after update:"
psql "$DB_URL" -v ON_ERROR_STOP=1 -c "SELECT key, updated_at, value FROM runtime_config WHERE key IN ('strategies','global');"

cat <<'MSG'

Next checks:
1. Keep trading-bot running.
2. Watch logs for: Runtime config reloaded from database
3. Check Prometheus metric bot_runtime_config_stale = 0.
4. Wait <= config_poll_interval_secs and confirm next scan uses the updated strategy threshold.
MSG
