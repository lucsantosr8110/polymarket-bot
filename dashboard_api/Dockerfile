FROM python:3.12-slim

# curl is used by the container healthcheck (GET /api/health).
RUN apt-get update && apt-get install -y --no-install-recommends curl && \
    rm -rf /var/lib/apt/lists/* && \
    adduser --disabled-password --gecos "" --uid 10001 appuser

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Code is imported as the `dashboard_api` package (uvicorn dashboard_api.main:app),
# so the build context (./dashboard_api) is copied into a same-named subdir.
COPY . ./dashboard_api

USER appuser:appuser
EXPOSE 8001
ENTRYPOINT ["uvicorn", "dashboard_api.main:app", "--host", "0.0.0.0", "--port", "8001"]
