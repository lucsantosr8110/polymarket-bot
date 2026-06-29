@echo off
cd /d "%~dp0"
echo Subindo stack (postgres, model-server, bot, copy-trading-bot, prometheus, grafana)...
docker compose up -d --build
echo.
docker compose ps
pause
